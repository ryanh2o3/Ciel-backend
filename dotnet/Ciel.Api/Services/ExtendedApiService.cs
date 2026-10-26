using Ciel.Api.Config;
using Ciel.Api.Http.Errors;
using StackExchange.Redis;
using System.Text;
using static Ciel.Api.Services.ApiData;
using static Ciel.Api.Services.ParitySql;

namespace Ciel.Api.Services;

/// <summary>Native social, safety, stories, search and moderation operations.</summary>
public sealed class ExtendedApiService(ApiData db, MediaService media, IConnectionMultiplexer redis,
                                       StorageService storage, ILogger<ExtendedApiService> logger)
{
    static void length(string? value, int max, string error)
    {
        if (value != null && System.Text.Encoding.UTF8.GetByteCount(value) > max)
            throw ApiException.BadRequest(error);
    }
    static void characters(string? value, int max, string error)
    {
        if (value != null && value.EnumerateRunes().Count() > max)
            throw ApiException.BadRequest(error);
    }
    private void changed(int count, string error)
    {
        if (count == 0)
            throw ApiException.NotFound(error);
    }
    private void avatar(Row row, string prefix, bool optional)
    {
        string? key = text(row, prefix + "avatar_key");
        string? url = media.GeneratePresignedGetUrlAsync(key, 14400, CancellationToken.None).GetAwaiter().GetResult();
        if (optional)
        {
            if (key == null)
                row.remove(prefix + "avatar_key");
            if (url != null)
                row.put(prefix + "avatar_url", url);
        }
        else
        {
            row.remove(prefix + "avatar_key");
            row.put(prefix + "avatar_url", url);
        }
    }
    private Row publicUser(Row row, bool resolve)
    {
        row.remove("email");
        if (resolve)
            avatar(row, "", false);
        else
        {
            row.remove("avatar_key");
            row.put("avatar_url", null);
        }
        row.put("followers_count", 0L);
        row.put("following_count", 0L);
        row.put("posts_count", 0L);
        return row;
    }
    private void post(Row row, bool primary)
    {
        row.put("visibility", "public".Equals(row.get("visibility")) ? "Public" : "FollowersOnly");
        avatar(row, "owner_", true);
        if (primary && row.get("media_ids") is Guid[] ids && ids.Length > 0)
        {
            var m = media.GetMediaAsync(ids[0], CancellationToken.None).GetAwaiter().GetResult();
            if (m != null)
                row.put("primary_media", m);
        }
    }
    private void story(Row row)
    {
        row.put("visibility", text(row, "visibility") switch {
            "public" => "Public",
            "friends_only" => "FriendsOnly",
            "close_friends_only" => "CloseFriendsOnly",
            _ => throw ApiException.Internal("internal server error"),
        });
        avatar(row, "user_", true);
        var m = media.GetMediaAsync(uuid(row, "media_id"), CancellationToken.None).GetAwaiter().GetResult();
        if (m != null)
            row.put("media", m);
    }
    public void invalidate(Guid user)
    {
        try
        {
            redis.GetDatabase().KeyDelete("feed:home:" + user);
            foreach (var endpoint in redis.GetEndPoints())
            {
                var server = redis.GetServer(endpoint);
                foreach (var key in server.Keys(pattern: "feed:stories:" + user + ":*", pageSize: 100))
                    redis.GetDatabase().KeyDelete(key);
            }
        }
        catch (Exception e)
        {
            logger.LogWarning(e, "feed invalidation failed");
        }
    }
    public void invalidateFollowers(Guid user)
    {
        foreach (var row in db.rows(social_list_follower_ids_0, user))
            invalidate(uuid(row, "follower_id"));
        invalidate(user);
    }
    public void activity(Guid user, string type)
    {
        try
        {
            int points = type switch {
                "post_created" => 5,
                "comment_created" => 2,
                "like_received" => 1,
                "follower_gained" => 3,
                "content_removed" => -25,
                _ => 0,
            };
            string? field = type switch {
                "post_created" => "posts_count",
                "comment_created" => "comments_count",
                "like_received" => "likes_received_count",
                "follower_gained" => "followers_count",
                _ => null,
            };
            db.execute("UPDATE user_trust_scores SET " +
                       "trust_points=GREATEST(0,trust_points+$1),updated_at=now(),last_activity_at=now()" +
                           (field == null ? "" : "," + field + "=" + field + "+1") + " WHERE user_id=$2",
                       points, user);
            if (Math.Abs(points) >= 5)
                recalculate(user);
        }
        catch (Exception e)
        {
            logger.LogWarning(e, "trust activity failed");
        }
    }
    private void recalculate(Guid user)
    {
        db.execute("UPDATE user_trust_scores SET trust_level=CASE WHEN strikes>=3 THEN 0 WHEN account_age_days>=90 " +
                   "AND posts_count>=50 AND trust_points>=200 AND flags_received<3 THEN 2 WHEN account_age_days>=7 " +
                   "AND posts_count>=5 AND trust_points>=20 AND flags_received<5 THEN 1 ELSE 0 END,updated_at=now() " +
                   "WHERE user_id=$1",
                   user);
    }
    public void notify(Guid recipient, Guid actor, string type, Row payload)
    {
        if (recipient.Equals(actor))
            return;
        try
        {
            var person = db.one(notifications_create_if_not_self_0, actor);
            if (person != null)
            {
                payload.put("actor_id", actor);
                payload.put("actor_handle", person.get("handle"));
                payload.put("actor_display_name", person.get("display_name"));
                string role = type switch { "user_followed" => "follower", "post_liked" => "liker",
                                            "post_commented" => "commenter",
                                            _ => "reactor" };
                payload.put(role + "_id", actor);
                payload.put(role + "_handle", person.get("handle"));
            }
            string json = System.Text.Json.JsonSerializer.Serialize(payload);
            db.execute("INSERT INTO notifications(user_id,notification_type,payload) VALUES($1,$2,$3::jsonb)",
                       recipient, type, json);
        }
        catch (Exception e)
        {
            logger.LogWarning(e, "notification failed");
        }
    }
    public Row updateProfile(Guid user, Guid id, string? display, string? bio, string? key)
    {
        if (!user.Equals(id))
            throw ApiException.Forbidden("cannot update other users");
        if (display != null && string.IsNullOrWhiteSpace(display))
            throw ApiException.BadRequest("display_name cannot be empty");
        length(display, 50, "display_name must be at most 50 characters");
        length(bio, 500, "bio must be at most 500 characters");
        var previous = db.one(users_get_user_0, id);
        var result = db.required(users_update_profile_0, "user not found", id, display, bio, key);
        if (previous != null && key != null && !key.Equals(previous.get("avatar_key")))
        {
            try
            {
                redis.GetDatabase().KeyDelete("presigned:" + previous.get("avatar_key"));
            }
            catch (Exception)
            {
            }
        }
        avatar(result, "", false);
        return result;
    }
    public void deleteAccount(Guid user)
    {
        db.transaction(() =>
                       {
                           changed(db.execute(users_delete_account_0, user), "user not found");
                           db.execute(users_delete_account_1, user);
                           db.execute(users_delete_account_2, user);
                           db.execute(users_delete_account_3, user);
                           return (object?)null;
                       });
    }
    public Row social(Guid user, Guid other, string action)
    {
        if (user.Equals(other))
            throw ApiException.BadRequest("cannot " + action + " yourself");
        bool result = db.transaction(
            () =>
            {
                if (action.Equals("follow"))
                {
                    db.required(social_follow_0, "user not found", other);
                    var count = db.one(social_follow_1, other);
                    if (Convert.ToInt64(count!.Values.First()) >= 5000)
                        throw ApiException.Forbidden("user has reached the follower limit");
                    return db.execute(social_follow_2, user, other) > 0;
                }
                if (action.Equals("block"))
                {
                    bool inserted = db.execute(social_block_0, user, other) > 0;
                    db.execute(social_block_1, user, other);
                    return inserted;
                }
                return db.execute(action.Equals("unfollow") ? social_unfollow_0 : social_unblock_0, user, other) > 0;
            });
        if (result)
        {
            invalidate(user);
            if (action.Equals("follow"))
            {
                notify(other, user, "user_followed", obj());
                activity(other, "follower_gained");
            }
        }
        return obj(
            action switch {
                "follow" => "followed",
                "unfollow" => "unfollowed",
                "block" => "blocked",
                _ => "unblocked",
            },
            result);
    }
    public Row relationship(Guid user, Guid other)
    {
        if (user.Equals(other))
            return obj("is_following", false, "is_followed_by", false, "is_blocking", false, "is_blocked_by", false);
        return db.single(social_relationship_status_0, user, other);
    }
    public Row socialList(Guid id, bool followers, int? size, string? cursor)
    {
        int n = limit(size);
        var rows = db.rows(followers ? (cursor == null ? social_list_followers_1 : social_list_followers_0)
                                     : (cursor == null ? social_list_following_1 : social_list_following_0),
                           cursorArgs(cursor, n, id));
        var result = page(rows, n, "followed_at", "id");
        List<Row> items = new();
        foreach (var row in rows)
        {
            object? followed = row.remove("followed_at");
            items.Add(obj("user", publicUser(row, false), "followed_at", followed));
        }
        result.put("items", items);
        return result;
    }
    public Row userPosts(Guid id, Guid? viewer, int? size, string? cursor)
    {
        int n = limit(size);
        string sql = viewer == null ? (cursor == null ? posts_list_by_user_3 : posts_list_by_user_2)
                                    : (cursor == null ? posts_list_by_user_1 : posts_list_by_user_0);
        object?[] args = viewer == null ? cursorArgs(cursor, n, id) : cursorArgs(cursor, n, id, viewer);
        var rows = db.rows(sql, args);
        var result = page(rows, n, "created_at", "id");
        foreach (var row in rows)
            post(row, true);
        return result;
    }
    public Row updatePost(Guid user, Guid id, string? caption)
    {
        length(caption, 2200, "caption must be at most 2200 characters");
        var row = db.required(posts_update_caption_0, "post not found", id, user, caption);
        post(row, false);
        return row;
    }
    public void deletePost(Guid user, Guid id)
    {
        changed(db.execute(posts_delete_post_0, id, user), "post not found");
    }
    private void visiblePost(Guid user, Guid id)
    {
        db.required(posts_can_view_post_0, "post not found", id, user);
    }
    public Row engagements(Guid user, Guid id, bool likes, int? size, string? cursor)
    {
        int n = limit(size);
        object?[] args = cursorArgs(cursor, n, id);
        visiblePost(user, id);
        return page(db.rows(likes ? (cursor == null ? engagement_list_likes_1 : engagement_list_likes_0)
                                  : (cursor == null ? engagement_list_comments_1 : engagement_list_comments_0),
                            args),
                    n, "created_at", "id");
    }
    public void deleteComment(Guid user, Guid post, Guid comment)
    {
        changed(db.execute(engagement_delete_comment_0, comment, post, user), "comment not found");
    }
    public void deleteMedia(Guid user, Guid id)
    {
        Row? row;
        try
        {
            row = db.one("DELETE FROM media WHERE id=$1 AND owner_id=$2 RETURNING original_key,thumb_key,medium_key",
                         id, user);
        }
        catch (Npgsql.PostgresException e) when (e.SqlState == "23503")
        {
            throw ApiException.Conflict("media is attached to a post or story and cannot be deleted");
        }
        if (row == null)
            throw ApiException.NotFound("media not found");
        foreach (var key in row.Values)
        {
            if (key is null) continue;
            try
            {
                redis.GetDatabase().KeyDelete("presigned:" + key);
                storage.DeleteAsync(key.ToString()!, CancellationToken.None).GetAwaiter().GetResult();
            }
            catch (Exception e)
            {
                logger.LogWarning(e, "media cleanup failed");
            }
        }
    }
    public Row notifications(Guid user, int? size, string? cursor)
    {
        int n = limit(size);
        return page(db.rows(cursor == null ? notifications_list_1 : notifications_list_0, cursorArgs(cursor, n, user)),
                    n, "created_at", "id");
    }
    public void markNotification(Guid user, Guid id)
    {
        changed(db.execute(notifications_mark_read_0, id, user), "notification not found");
    }
    public Row search(bool users, string? term, int? size, string? cursor)
    {
        if (term == null)
            throw ApiException.BadRequest("missing required parameter 'q'");
        term = term.Trim();
        if (System.Text.Encoding.UTF8.GetByteCount(term) < 2)
            throw ApiException.BadRequest("q must be at least 2 characters");
        length(term, 100, "q must be at most 100 characters");
        int n = limit(size);
        string pattern = "%" + term.Replace("\\", "\\\\").Replace("%", "\\%").Replace("_", "\\_") + "%";
        var rows = db.rows(users ? (cursor == null ? search_search_users_1 : search_search_users_0)
                                 : (cursor == null ? search_search_posts_1 : search_search_posts_0),
                           cursorArgs(cursor, n, pattern));
        var result = page(rows, n, "created_at", "id");
        foreach (var row in rows)
        {
            if (users)
                publicUser(row, true);
            else
                post(row, false);
        }
        return result;
    }
    public Row flag(Guid user, Guid id, bool post, string? reason)
    {
        Guid owner =
            post ? uuid(db.required("SELECT owner_id FROM posts WHERE id=$1", "post not found", id), "owner_id") : id;
        if (user.Equals(owner))
            throw ApiException.BadRequest(post ? "cannot report your own post" : "cannot report yourself");
        Row result;
        try
        {
            result = db.transaction(
                () =>
                {
                    var row = db.single(post ? moderation_flag_post_0 : moderation_flag_user_0, user, id, reason);
                    db.execute(post ? moderation_flag_post_1 : moderation_flag_user_1, user, id, reason);
                    return row;
                });
        }
        catch (Npgsql.PostgresException e)
        {
            string state = e.SqlState;
            if ("23505".Equals(state))
                throw ApiException.Conflict("you have already reported this " + (post ? "post" : "user"));
            if ("23503".Equals(state))
                throw ApiException.NotFound(post ? "post not found" : "user not found");
            throw;
        }
        recordFlag(owner);
        return result;
    }
    private void recordFlag(Guid user)
    {
        try
        {
            db.transaction(() =>
                           {
                               var row = db.one(trust_record_flag_0, user);
                               if (row == null)
                                   return (object?)null;
                               int flags = integer(row, "flags_received");
                               if (flags >= 10 && flags % 10 == 0)
                               {
                                   int strikes = integer(db.single(trust_add_strike_with_tx_0, user), "strikes");
                                   if (strikes >= 3)
                                       db.execute(trust_add_strike_with_tx_1,
                                                  DateTimeOffset.UtcNow.AddDays(strikes == 3   ? 7
                                                                                : strikes == 4 ? 30
                                                                                               : 365),
                                                  user);
                               }
                               recalculate(user);
                               return (object?)null;
                           });
        }
        catch (Exception e)
        {
            logger.LogWarning(e, "flag trust update failed");
        }
    }
    public void takedown(Guid? actor, Guid id, bool post, string? reason)
    {
        Guid owner = db.transaction(
            () =>
            {
                var row = db.required(post ? moderation_takedown_post_0 : moderation_takedown_comment_0,
                                      post ? "post not found" : "comment not found", id);
                db.execute(post ? moderation_takedown_post_1 : moderation_takedown_comment_1, actor, id, reason);
                return uuid(row, post ? "owner_id" : "user_id");
            });
        activity(owner, "content_removed");
    }
    public Row audit(int? size, string? cursor)
    {
        int n = limit(size);
        return page(db.rows(cursor == null ? moderation_list_audit_1 : moderation_list_audit_0, cursorArgs(cursor, n)),
                    n, "created_at", "id");
    }
    public Row trust(Guid user)
    {
        var row = db.required(trust_get_trust_score_0, "trust score not found", user);
        int level = integer(row, "trust_level");
        string name = level switch {
            1 => "Basic",
            2 => "Trusted",
            3 => "Verified",
            _ => "New",
        };
        return obj("user_id", user, "trust_level", level, "trust_level_name", name, "trust_points",
                   row.get("trust_points"), "account_age_days", row.get("account_age_days"), "posts_count",
                   row.get("posts_count"), "followers_count", row.get("followers_count"), "strikes", row.get("strikes"),
                   "is_banned",
                   row.get("banned_until") != null && (instant(row, "banned_until") > DateTimeOffset.UtcNow));
    }
    public Row quotas(Guid user)
    {
        var score = trust(user);
        var limits = RateLimits.ForTrustLevel(TrustLevelExtensions.FromInt32(integer(score, "trust_level")));
        var remaining = obj();
        string[] actions = { "post", "follow", "like", "comment", "media_read", "media_upload" };
        string[] fields = { "posts", "follows", "likes", "comments", "media_read", "media_upload" };
        long[] maxima = { limits.PostsPerHour,    limits.FollowsPerHour,   limits.LikesPerHour,
                          limits.CommentsPerHour, limits.MediaReadPerHour, limits.MediaUploadPerHour };
        for (int i = 0; i < actions.Length; i++)
        {
            var count = redis.GetDatabase().StringGet("ratelimit:" + user + ":" + actions[i] + ":" +
                                                      (DateTimeOffset.UtcNow.ToUnixTimeSeconds() / 3600));
            remaining.put(fields[i], Math.Max(0, maxima[i] - (count.IsNull ? 0 : (long)count)));
        }
        return obj("trust_level", score.get("trust_level_name"), "posts_per_hour", limits.PostsPerHour, "posts_per_day",
                   limits.PostsPerDay, "follows_per_hour", limits.FollowsPerHour, "follows_per_day",
                   limits.FollowsPerDay, "likes_per_hour", limits.LikesPerHour, "comments_per_hour",
                   limits.CommentsPerHour, "media_read_per_hour", limits.MediaReadPerHour, "media_upload_per_hour",
                   limits.MediaUploadPerHour, "remaining", remaining);
    }
    public void registerDevice(Guid? user, string? fingerprint, string? agent)
    {
        if (fingerprint == null || string.IsNullOrWhiteSpace(fingerprint))
            throw ApiException.BadRequest("fingerprint is required");
        length(fingerprint, 512, "fingerprint must be at most 512 characters");
        string hash = CryptoService.Sha256Hex(fingerprint);
        db.transaction(() =>
                       {
                           db.execute(fingerprint_register_fingerprint_0, hash, new Guid[0], agent);
                           var row = db.single(fingerprint_register_fingerprint_1, hash);
                           if (true.Equals(row.get("is_blocked")))
                               throw ApiException.Forbidden("This device has been blocked");
                           List<Guid> ids = new();
                           foreach (var id in (Guid[])row.get("user_ids")!)
                               ids.Add(Guid.Parse(id.ToString()!));
                           if (user != null && !ids.Contains(user.Value))
                           {
                               int count = integer(row, "account_count");
                               int risk = Math.Min(100, integer(row, "risk_score") + (count <= 2    ? 5
                                                                                      : count <= 5  ? 15
                                                                                      : count <= 10 ? 30
                                                                                                    : 50));
                               ids.Add(user.Value);
                               db.execute(fingerprint_register_fingerprint_2, ids.ToArray(), ids.Count, risk, agent,
                                          hash);
                           }
                           else
                               db.execute(fingerprint_register_fingerprint_3, agent, hash);
                           return (object?)null;
                       });
    }
    public List<Row> devices(Guid user)
    {
        var rows = db.rows(fingerprint_get_user_devices_0, user);
        foreach (var row in rows)
        {
            row.remove("user_ids");
            row.put("fingerprint_hash", text(row, "fingerprint_hash")!.Substring(0, 16));
        }
        return rows;
    }
    private static int maxInvites(int level)
    {
        return level switch {
            1 => 10,
            2 => 50,
            3 => 200,
            _ => 3,
        };
    }
    public List<Row> invites(Guid user)
    {
        return db.rows(invites_list_user_invites_0, user);
    }
    public Row inviteStats(Guid user)
    {
        var row = db.required(invites_get_invite_stats_0, "trust score not found", user);
        int max = maxInvites(integer(row, "trust_level"));
        return obj("invites_sent", row.get("invites_sent"), "successful_invites", row.get("successful_invites"),
                   "remaining_invites", Math.Max(0, max - integer(row, "invites_sent")), "max_invites", max);
    }
    public Row validateInvite(string code)
    {
        var row = db.one(invites_validate_invite_0, code);
        return obj("is_valid", row != null && true.Equals(row.get("is_valid")) &&
                                   (instant(row, "expires_at") > DateTimeOffset.UtcNow) &&
                                   integer(row, "use_count") < integer(row, "max_uses"));
    }
    public Row createInvite(Guid? user, long days)
    {
        int maxDays = user == null ? 365 : 30;
        if (days < 1 || days > maxDays)
            throw ApiException.BadRequest("days_valid must be between 1 and " + maxDays);
        return db.transaction(
            () =>
            {
                if (user != null)
                {
                    var quota = db.one(invites_create_invite_0, user);
                    if (quota == null)
                        throw ApiException.Internal("internal server error");
                    int max = maxInvites(integer(quota, "trust_level"));
                    if (integer(quota, "invites_sent") >= max)
                        throw ApiException.Forbidden("Maximum invite limit reached for your trust level (" + max + ")");
                }
                string alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                for (int attempt = 0; attempt < 10; attempt++)
                {
                    var code = new System.Text.StringBuilder();
                    for (int i = 0; i < 12; i++)
                        code.Append(
                            alphabet[System.Security.Cryptography.RandomNumberGenerator.GetInt32(alphabet.Length)]);
                    var row = db.one(
                        "INSERT INTO invite_codes(code,created_by,expires_at,invite_type) VALUES($1,$2,$3,$4) ON " +
                        "CONFLICT DO NOTHING RETURNING " +
                        "code,created_by,used_by,created_at,used_at,expires_at,is_valid,invite_type,use_count,max_uses",
                        code.ToString()!, user, DateTimeOffset.UtcNow.AddDays(days),
                        user == null ? "admin" : "standard");
                    if (row != null)
                    {
                        if (user != null)
                            db.execute(invites_create_invite_2, user);
                        return row;
                    }
                }
                throw ApiException.Internal("internal server error");
            });
    }
    public void revokeInvite(Guid user, string code)
    {
        changed(db.execute(invites_revoke_invite_0, code, user), "invite code not found or already used");
    }
    public Row createStory(Guid user, Guid mediaId, string? caption, string visibility)
    {
        characters(caption, 2200, "caption exceeds 2200 characters");
        string? value = visibility switch {
            "Public" => "public",
            "FriendsOnly" => "friends_only",
            "CloseFriendsOnly" => "close_friends_only",
            _ => throw ApiException.BadRequest("invalid request body"),
        };
        var owner = db.required(stories_create_story_0, "media not found", mediaId);
        if (!user.Equals(uuid(owner, "owner_id")))
            throw ApiException.Forbidden("media does not belong to you");
        var row = db.single(stories_create_story_1, user, mediaId, caption, DateTimeOffset.UtcNow.AddHours(24), value);
        story(row);
        invalidateFollowers(user);
        return row;
    }
    private Row visibleStory(Guid user, Guid id)
    {
        return db.required(stories_get_story_0, "story not found", id, DateTimeOffset.UtcNow, user);
    }
    public Row getStory(Guid user, Guid id)
    {
        var row = visibleStory(user, id);
        story(row);
        return row;
    }
    public Row stories(Guid user, Guid? owner, int? size, string? cursor)
    {
        int n = limit(size);
        List<Row> rows;
        if (owner != null)
            rows = db.rows(cursor == null ? stories_get_user_stories_1 : stories_get_user_stories_0,
                           cursorArgs(cursor, n, owner, DateTimeOffset.UtcNow, user));
        else
            rows = db.rows(cursor == null ? stories_get_stories_feed_1 : stories_get_stories_feed_0,
                           cursorArgs(cursor, n, DateTimeOffset.UtcNow, user));
        var result = page(rows, n, "created_at", "id");
        foreach (var row in rows)
            story(row);
        return result;
    }
    public void deleteStory(Guid user, Guid id)
    {
        changed(db.execute(stories_delete_story_0, id, user), "story not found");
        invalidateFollowers(user);
    }
    public Row viewers(Guid user, Guid id, int? size, string? cursor)
    {
        int n = limit(size);
        object?[] args = cursorArgs(cursor, n, id);
        var owner = db.required(stories_get_story_owner_0, "story not found", id);
        if (!user.Equals(uuid(owner, "user_id")))
            throw ApiException.Forbidden("only the story owner can view viewers");
        var rows = db.rows(cursor == null ? stories_list_viewers_1 : stories_list_viewers_0, args);
        var result = page(rows, n, "viewed_at", "viewer_id");
        foreach (var row in rows)
            avatar(row, "viewer_", true);
        return result;
    }
    public Row react(Guid user, Guid id, string emoji)
    {
        if (emoji == null || string.IsNullOrWhiteSpace(emoji))
            throw ApiException.BadRequest("emoji cannot be empty");
        emoji = emoji.Trim();
        characters(emoji, 7, "emoji must be at most 7 characters");
        var owner = visibleStory(user, id);
        var row = db.single(stories_add_reaction_0, id, user, emoji);
        row.remove("is_new");
        notify(uuid(owner, "user_id"), user, "story_reaction", obj("story_id", id, "emoji", emoji));
        return row;
    }
    public Row reactions(Guid user, Guid id, int? size, string? cursor)
    {
        int n = limit(size);
        object?[] args = cursorArgs(cursor, n, id);
        visibleStory(user, id);
        return page(db.rows(cursor == null ? stories_list_reactions_1 : stories_list_reactions_0, args), n,
                    "created_at", "id");
    }
    public void removeReaction(Guid user, Guid id)
    {
        changed(db.execute(stories_remove_reaction_0, id, user), "reaction not found");
    }
    public void seen(Guid user, Guid id)
    {
        visibleStory(user, id);
        db.execute(stories_mark_seen_0, id, user);
    }
    public Row metrics(Guid user, Guid id)
    {
        var row = db.required(stories_get_metrics_0, "story not found", id, user);
        row.put("story_id", id);
        row.put("reactions_by_emoji", db.rows(stories_get_metrics_1, id));
        row.put("viewer_ids", db.rows(stories_get_metrics_2, id).Select(r => r.get("viewer_id")).ToList());
        return row;
    }
    public Row highlight(Guid user, Guid id, string name)
    {
        if (name == null || string.IsNullOrWhiteSpace(name))
            throw ApiException.BadRequest("highlight_name cannot be empty");
        characters(name, 50, "highlight_name must be at most 50 characters");
        return db.transaction(() =>
                              {
                                  var owner = db.required(stories_add_to_highlight_0, "story not found", id);
                                  if (!user.Equals(uuid(owner, "user_id")))
                                      throw ApiException.NotFound("story not found");
                                  var row = db.single(stories_add_to_highlight_1, user, name, id);
                                  db.execute(stories_add_to_highlight_2, uuid(row, "id"), id);
                                  return row;
                              });
    }
    public Row highlights(Guid user, int? size, string? cursor)
    {
        int n = limit(size);
        return page(db.rows(cursor == null ? stories_get_user_highlights_1 : stories_get_user_highlights_0,
                            cursorArgs(cursor, n, user)),
                    n, "created_at", "id");
    }
    public void postEngagement(Guid user, Guid id, string type, string? body)
    {
        var owner = db.one("SELECT owner_id FROM posts WHERE id=$1", id);
        if (owner == null)
            return;
        var payload = obj("post_id", id);
        if (body != null)
            payload.put("comment_preview", string.Concat(body.EnumerateRunes().Take(100).Select(r => r.ToString())));
        notify(uuid(owner, "owner_id"), user, type, payload);
        if (type == "post_liked")
            activity(uuid(owner, "owner_id"), "like_received");
        else
            activity(user, "comment_created");
    }
}
