#!/usr/bin/env python3
"""Exercise each native API against disposable local Postgres/Redis and an AWS test double.

Build all APIs first. Requires initdb/postgres/psql, redis-server/redis-cli, Java 21,
and dotnet on PATH (PG_BIN and JAVA_BIN can override tool locations).
Never connects to a configured deployment: ports, database, credentials, and keys
are generated here. AWS assertions cover HTTP contracts, not pixel processing.
"""
import argparse, base64, hashlib, http.server, json, os, pathlib, re, shutil
import socket, subprocess, tempfile, threading, time, urllib.request, urllib.error, uuid

ROOT = pathlib.Path(__file__).resolve().parents[1]
PASSWORD = 'ParityOnly-Password-2026!'
ADMIN = 'local-parity-admin'
PROCESSES = []
LOGS = []

def port():
    with socket.socket() as s:
        s.bind(('127.0.0.1', 0))
        return s.getsockname()[1]

def start(name, args, env=None):
    log = open(WORK / (name + '.log'), 'w')
    LOGS.append(log)
    proc = subprocess.Popen([str(a) for a in args], cwd=ROOT, env=env, stdout=log, stderr=log)
    PROCESSES.append(proc)
    return proc

def command(args, data=None):
    result = subprocess.run([str(a) for a in args], input=data, text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError(result.stderr[-3000:])
    return result.stdout

def sql(query):
    return command([PG / 'psql', DB, '-X', '-qAt', '-v', 'ON_ERROR_STOP=1'], query).strip()

class AwsDouble(http.server.BaseHTTPRequestHandler):
    """Local S3/SQS protocol double. Records queue jobs for payload assertions."""
    jobs = []
    def log_message(self, *args): pass
    def do_DELETE(self): self.reply(204, b'')
    def do_PUT(self):
        self.rfile.read(int(self.headers.get('Content-Length', 0)))
        self.reply(200, b'')
    def do_HEAD(self): self.reply(200, b'')
    def do_POST(self):
        data = self.rfile.read(int(self.headers.get('Content-Length', 0)))
        target = self.headers.get('X-Amz-Target', '').split('.')[-1]
        try: payload = json.loads(data)
        except ValueError: payload = {}
        if target == 'SendMessage':
            body = payload.get('MessageBody', '')
            self.jobs.append(json.loads(body))
            result = {'MessageId': str(uuid.uuid4()), 'MD5OfMessageBody': hashlib.md5(body.encode()).hexdigest()}
        elif target in ('GetQueueUrl', 'CreateQueue'): result = {'QueueUrl': AWS + '/queue'}
        else: result = {}
        self.reply(200, json.dumps(result).encode())
    def reply(self, status, data):
        self.send_response(status)
        self.send_header('Content-Type', 'application/x-amz-json-1.0')
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)

def request(base, method, route, data=None, token=None, admin=False, headers=None):
    h = {'Content-Type': 'application/json', **(headers or {})}
    if token: h['Authorization'] = 'Bearer ' + token
    if admin: h['x-admin-token'] = ADMIN
    req = urllib.request.Request(base + route, method=method, headers=h,
                                 data=None if data is None else data if isinstance(data, bytes) else json.dumps(data).encode())
    try:
        res = urllib.request.urlopen(req, timeout=15)
    except urllib.error.HTTPError as error:
        res = error
    body = res.read().decode()
    try: body = json.loads(body)
    except ValueError: pass
    return res.status, body, dict(res.headers)

def wait_api(name, base, proc):
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        if proc.poll() is not None: break
        try:
            if request(base, 'GET', '/health')[0] == 200: return
        except (OSError, urllib.error.URLError): pass
        time.sleep(.2)
    raise RuntimeError(f'{name} failed to start; see {WORK / (name + ".log")}')

def normalize(value, key=''):
    if isinstance(value, dict): return {k: normalize(v, k) for k, v in sorted(value.items())}
    if isinstance(value, list): return [normalize(v, key) for v in value]
    if isinstance(value, str):
        if key in ('access_token', 'refresh_token'): return '<token>'
        if key == 'code': return '<invite>'
        if key == 'next_cursor': return '<cursor>'
        if re.fullmatch(r'[0-9a-f-]{36}', value): return '<uuid>'
        if re.match(r'^\d{4}-\d\d-\d\dT', value): return '<timestamp>'
        if value.startswith('http'): return '<url>'
        if key in ('object_key', 'original_key', 'thumb_key', 'medium_key'): return re.sub(r'[0-9a-f-]{36}', '<uuid>', value)
    return value

def reset():
    # Only the database initialized by this process is accessible through DB.
    sql('TRUNCATE users,invite_codes,device_fingerprints,moderation_actions CASCADE;')
    command(['redis-cli', '-p', REDIS_PORT, 'FLUSHDB'])

class Suite:
    def __init__(self, backend, base):
        self.backend, self.base, self.results = backend, base, {}
    def check(self, label, method, route, data=None, token=None, status=200, admin=False):
        actual, body, headers = request(self.base, method, route, data, token, admin)
        if actual != status:
            raise AssertionError(f'{self.backend} {label}: expected {status}, got {actual}: {body}')
        served = next((v for k,v in headers.items() if k.lower() == 'x-ciel-served-by'), None)
        assert served == self.backend, (label, served)
        self.results[label] = (actual, normalize(body))
        return body
    def signup(self, handle):
        invite = self.check('admin invite '+handle,'POST','/v1/admin/invites',{},admin=True)
        return self.check('signup '+handle,'POST','/v1/users',{
            'handle':handle,'email':handle+'@parity.test','display_name':'Parity '+handle,
            'password':PASSWORD,'invite_code':invite['code']})
    def login(self, handle):
        return self.check('login '+handle,'POST','/v1/auth/login',{'email':handle+'@parity.test','password':PASSWORD})
    def run(self):
        c=self.check
        c('health','GET','/health')
        c('metrics auth','GET','/metrics',status=403)
        code,metrics,_=request(self.base,'GET','/metrics',admin=True)
        assert code==200 and 'http_requests_total' in metrics
        c('missing bearer','GET','/v1/auth/me',status=401)
        c('invalid bearer','GET','/v1/auth/me',token='invalid',status=401)
        c('bad login','POST','/v1/auth/login',{'email':'absent@parity.test','password':PASSWORD},status=401)
        c('invalid refresh','POST','/v1/auth/refresh',{'refresh_token':'invalid'},status=401)
        c('invalid revoke','POST','/v1/auth/revoke',{'refresh_token':'invalid'},status=204)
        for payload in [{}, {'email':None,'password':PASSWORD}, [], 'invalid', b'{bad json']:
            c('malformed login '+str(payload),'POST','/v1/auth/login',payload,status=400)
        c('invalid path UUID','GET','/v1/users/not-a-uuid',status=400)
        c('unknown invite','GET','/v1/invites/validate/not-a-code')
        c('missing admin','POST','/v1/admin/invites',{},status=403)
        valid={'handle':'validation_user','email':'validation@parity.test','display_name':'Validation','password':PASSWORD,'invite_code':'invalid'}
        for field,value in [('handle','x'),('handle','x'*31),('handle','bad!'),('email','x@x.@'),('display_name','é'*26),('bio','é'*251),('password','Aa'+'é'*64),('password','Password١'),('password','short'),('invite_code','')]:
            c('invalid '+field+' '+value,'POST','/v1/users',dict(valid,**{field:value}),status=400)
        alice=self.signup('parity_alice'); bob=self.signup('parity_bob')
        aid,bid=alice['id'],bob['id']
        pair=self.login('parity_alice'); a=pair['access_token']; b=self.login('parity_bob')['access_token']
        c('padded login','POST','/v1/auth/login',{'email':' parity_alice@parity.test ','password':PASSWORD},status=401)
        c('unsupported upload','POST','/v1/media/upload',{'content_type':'application/pdf','bytes':10},a,400)
        c('invalid query integer','GET','/v1/search/users?limit=abc&q=alice',token=a,status=400)
        c('me','GET','/v1/auth/me',token=a)
        c('public user','GET',f'/v1/users/{aid}')
        c('profile forbidden','PATCH',f'/v1/users/{bid}',{},a,403)
        c('profile update','PATCH',f'/v1/users/{aid}',{'bio':'hello','display_name':'Alice'},a)
        c('trust','GET','/v1/account/trust-score',token=a)
        c('quotas','GET','/v1/account/rate-limits',token=a)
        c('device register','POST','/v1/account/device/register',{'fingerprint':'local-device'},a,204)
        c('device list','GET','/v1/account/devices',token=a)
        c('anonymous device','POST','/v1/account/device/register',{'fingerprint':'anonymous-device'},status=204)
        invite=c('create invite','POST','/v1/invites',{},a)
        c('validate invite','GET','/v1/invites/validate/'+invite['code'])
        c('list invites','GET','/v1/invites',token=a)
        c('invite stats','GET','/v1/invites/stats',token=a)
        c('revoke invite','POST','/v1/invites/'+invite['code']+'/revoke',token=a,status=204)
        c('revoked invite','GET','/v1/invites/validate/'+invite['code'])
        c('self follow','POST',f'/v1/users/{aid}/follow',token=a,status=400)
        c('follow','POST',f'/v1/users/{bid}/follow',token=a)
        c('follow duplicate','POST',f'/v1/users/{bid}/follow',token=a)
        c('follow back','POST',f'/v1/users/{aid}/follow',token=b)
        c('relationship','GET',f'/v1/users/{bid}/relationship',token=a)
        c('followers','GET',f'/v1/users/{bid}/followers',token=a)
        c('following','GET',f'/v1/users/{aid}/following',token=a)
        media_id=str(uuid.uuid4()); spare=str(uuid.uuid4())
        for mid in (media_id,spare):
            sql(f"INSERT INTO media(id,owner_id,original_key,thumb_key,medium_key,width,height,bytes) VALUES('{mid}','{aid}','original.jpg','thumb.jpg','medium.jpg',100,100,1234);")
        c('missing upload bytes','POST','/v1/media/upload',{'content_type':'image/jpeg'},a,400)
        c('null media ids','POST','/v1/posts',{'media_ids':None},a,400)
        c('unicode caption limit','POST','/v1/posts',{'media_ids':[str(uuid.uuid4())],'caption':'é'*1101},a,400)
        upload=c('upload','POST','/v1/media/upload',{'content_type':'image/jpeg','bytes':1234},a)
        c('upload status','GET',f'/v1/media/upload/{upload["upload_id"]}/status',token=a)
        c('complete upload','POST',f'/v1/media/upload/{upload["upload_id"]}/complete',token=a,status=202)
        c('media read','GET',f'/v1/media/{media_id}',token=a)
        c('private media','GET',f'/v1/media/{media_id}',token=b,status=404)
        c('media delete','DELETE',f'/v1/media/{spare}',token=a,status=204)
        post=c('create post','POST','/v1/posts',{'media_ids':[media_id],'caption':'hello parity'},a); pid=post['id']
        c('get post','GET',f'/v1/posts/{pid}',token=a)
        c('list posts','GET',f'/v1/users/{aid}/posts',token=b)
        c('update post','PATCH',f'/v1/posts/{pid}',{'caption':'updated parity'},a)
        c('like','POST',f'/v1/posts/{pid}/like',token=b)
        c('duplicate like','POST',f'/v1/posts/{pid}/like',token=b)
        c('likes','GET',f'/v1/posts/{pid}/likes',token=a)
        c('unicode comment limit','POST',f'/v1/posts/{pid}/comment',{'body':'😀'*1001},b,400)
        comment=c('comment','POST',f'/v1/posts/{pid}/comment',{'body':'nice picture'},b)
        c('comments','GET',f'/v1/posts/{pid}/comments',token=a)
        c('home feed','GET','/v1/feed',token=b)
        c('refresh feed','POST','/v1/feed/refresh',token=b,status=204)
        c('search users','GET','/v1/search/users?q=parity',token=a)
        c('search posts','GET','/v1/search/posts?q=parity',token=a)
        c('bad limit','GET','/v1/notifications?limit=0',token=a,status=400)
        c('bad cursor','GET','/v1/notifications?cursor=bad',token=a,status=400)
        story=c('create story','POST','/v1/stories',{'media_id':media_id,'caption':'story','visibility':'FriendsOnly'},a); sid=story['id']
        c('get story','GET',f'/v1/stories/{sid}',token=b)
        c('user stories','GET',f'/v1/users/{aid}/stories',token=b)
        c('stories feed','GET','/v1/feed/stories',token=b)
        c('seen','POST',f'/v1/stories/{sid}/seen',token=b,status=204)
        c('seen twice','POST',f'/v1/stories/{sid}/seen',token=b,status=204)
        c('react','POST',f'/v1/stories/{sid}/reactions',{'emoji':'❤️'},b)
        c('replace reaction','POST',f'/v1/stories/{sid}/reactions',{'emoji':'👍'},b)
        c('reactions','GET',f'/v1/stories/{sid}/reactions',token=a)
        c('viewers','GET',f'/v1/stories/{sid}/viewers',token=a)
        c('viewers forbidden','GET',f'/v1/stories/{sid}/viewers',token=b,status=403)
        metrics=c('story metrics','GET',f'/v1/stories/{sid}/metrics',token=a)
        assert metrics['view_count']==metrics['reaction_count']==1
        c('highlight','POST',f'/v1/stories/{sid}/highlights',{'highlight_name':'Favorites'},a)
        c('highlights','GET',f'/v1/users/{aid}/highlights',token=b)
        c('remove reaction','DELETE',f'/v1/stories/{sid}/reactions',token=b,status=204)
        c('media conflict','DELETE',f'/v1/media/{media_id}',token=a,status=409)
        c('block','POST',f'/v1/users/{bid}/block',token=a)
        c('blocked story','GET',f'/v1/stories/{sid}',token=b,status=404)
        c('blocked post','GET',f'/v1/posts/{pid}',token=b,status=404)
        c('unblock','POST',f'/v1/users/{bid}/unblock',token=a)
        c('unfollow','POST',f'/v1/users/{bid}/unfollow',token=a)
        # Rust notification delivery is asynchronous. Poll without recording nondeterministic timing.
        deadline=time.monotonic()+5
        while time.monotonic()<deadline:
            result=request(self.base,'GET','/v1/notifications',token=a)[1]
            if len(result.get('items',[]))>=5:break
            time.sleep(.05)
        notifications=c('notifications','GET','/v1/notifications',token=a)
        assert len(notifications['items'])>=5
        c('mark notification','POST',f'/v1/notifications/{notifications["items"][0]["id"]}/read',token=a,status=204)
        c('flag user','POST',f'/v1/moderation/users/{aid}/flag',{'reason':'test'},b)
        c('duplicate flag','POST',f'/v1/moderation/users/{aid}/flag',{'reason':'test'},b,409)
        c('flag post','POST',f'/v1/moderation/posts/{pid}/flag',{'reason':'test'},b)
        c('takedown comment','POST',f'/v1/moderation/comments/{comment["id"]}/takedown',{},admin=True,status=204)
        comment=c('second comment','POST',f'/v1/posts/{pid}/comment',{'body':'another'},b)
        c('delete comment','DELETE',f'/v1/posts/{pid}/comments/{comment["id"]}',token=b,status=204)
        c('unlike','DELETE',f'/v1/posts/{pid}/like',token=b,status=204)
        c('delete story','DELETE',f'/v1/stories/{sid}',token=a,status=204)
        c('takedown post','POST',f'/v1/moderation/posts/{pid}/takedown',{},admin=True,status=204)
        c('audit','GET','/v1/moderation/audit',admin=True)
        post=c('second post','POST','/v1/posts',{'media_ids':[media_id],'caption':'delete me'},a)
        c('delete post','DELETE',f'/v1/posts/{post["id"]}',token=a,status=204)
        pair=c('refresh','POST','/v1/auth/refresh',{'refresh_token':pair['refresh_token']})
        c('revoke','POST','/v1/auth/revoke',{'refresh_token':pair['refresh_token']},status=204)
        c('revoked refresh','POST','/v1/auth/refresh',{'refresh_token':pair['refresh_token']},status=401)
        c('delete account','DELETE','/v1/account',token=a,status=204)
        c('deleted user token','GET','/v1/auth/me',token=a,status=401)
        return self.results

def run(backends):
    global WORK, PG, DB, AWS, REDIS_PORT
    WORK=pathlib.Path(tempfile.mkdtemp(prefix='ciel-parity-'))
    print('Test logs:',WORK,flush=True)
    pg_override=os.environ.get('PG_BIN')
    PG=pathlib.Path(pg_override or (pathlib.Path(shutil.which('initdb')).parent if shutil.which('initdb') else '/opt/homebrew/opt/postgresql@16/bin'))
    pgport,REDIS_PORT,awsport=port(),port(),port()
    DB=f'postgresql://parity@127.0.0.1:{pgport}/parity'
    command([PG/'initdb','-D',WORK/'pg','-U','parity','-A','trust','--no-locale','--encoding=UTF8'])
    start('postgres',[PG/'postgres','-D',WORK/'pg','-h','127.0.0.1','-p',pgport,'-k',WORK])
    start('redis',['redis-server','--bind','127.0.0.1','--port',REDIS_PORT,'--save','','--appendonly','no'])
    for attempt in range(100):
        try: command([PG/'createdb','-h','127.0.0.1','-p',pgport,'-U','parity','parity']);break
        except RuntimeError: time.sleep(.1)
    for migration in sorted((ROOT/'migrations').glob('*.sql')): sql(migration.read_text())
    AWS=f'http://127.0.0.1:{awsport}'
    server=http.server.ThreadingHTTPServer(('127.0.0.1',awsport),AwsDouble)
    threading.Thread(target=server.serve_forever,daemon=True).start()
    env={**os.environ,'DATABASE_URL':DB,'REDIS_URL':f'redis://127.0.0.1:{REDIS_PORT}',
         'PASETO_ACCESS_KEY':base64.b64encode(os.urandom(32)).decode(),
         'PASETO_REFRESH_KEY':base64.b64encode(os.urandom(32)).decode(),
         'ADMIN_TOKEN':ADMIN,'APP_MODE':'api','S3_ENDPOINT':AWS,'S3_REGION':'us-east-1','S3_BUCKET':'parity',
         'S3_FORCE_PATH_STYLE':'true','QUEUE_ENDPOINT':AWS,'QUEUE_REGION':'us-east-1','QUEUE_NAME':'parity',
         'S3_ACCESS_KEY':'test','S3_SECRET_KEY':'test','SQS_ACCESS_KEY':'test','SQS_SECRET_KEY':'test',
         'AWS_ACCESS_KEY_ID':'test','AWS_SECRET_ACCESS_KEY':'test','AWS_REGION':'us-east-1',
         'AWS_EC2_METADATA_DISABLED':'true','IP_SIGNUP_RATE_LIMIT':'1000','RUST_LOG':'warn'}
    urls={}
    for name in backends:
        httpport=port();urls[name]=f'http://127.0.0.1:{httpport}'
        settings={**env,'HTTP_ADDR':f'127.0.0.1:{httpport}','SERVER_PORT':str(httpport),'ASPNETCORE_URLS':urls[name],'CIEL_SERVED_BY':name}
        java=os.environ.get('JAVA_BIN','/opt/homebrew/opt/openjdk@21/bin/java' if pathlib.Path('/opt/homebrew/opt/openjdk@21/bin/java').exists() else 'java')
        args={'rust':[ROOT/'target/debug/ciel'],'spring':[java,'-jar',ROOT/'spring/target/ciel-backend-spring.jar'],
              'dotnet':['dotnet',ROOT/'dotnet/Ciel.Api/bin/Debug/net8.0/Ciel.Api.dll']}[name]
        proc=start(name,args,settings);wait_api(name,urls[name],proc)
    results={}
    for name,base in urls.items():
        reset();suite=Suite(name,base);results[name]=suite.run()
        print(f'{name}: {len(results[name])} functional checks passed',flush=True)
    reference=backends[0]
    differences=[]
    for name in backends[1:]:
        for label,expected in results[reference].items():
            actual=results[name][label]
            if expected!=actual: differences.append((name,label,expected,actual))
    (WORK/'results.json').write_text(json.dumps(results,indent=2))
    if differences:
        for name,label,expected,actual in differences:print(f'DIFF {reference} vs {name}: {label}\n  expected={expected}\n  actual={actual}',flush=True)
        raise AssertionError(f'{len(differences)} response differences')
    # Same account/database/keys: every issuer must work at every verifier.
    reset();setup=Suite(reference,urls[reference]);setup.signup('parity_cross')
    checks=0
    for issuer in backends:
        for verifier in backends:
            pair=Suite(issuer,urls[issuer]).login('parity_cross')
            code,body,_=request(urls[verifier],'GET','/v1/auth/me',token=pair['access_token'])
            assert code==200,(issuer,verifier,code,body)
            code,body,_=request(urls[verifier],'POST','/v1/auth/refresh',{'refresh_token':pair['refresh_token']})
            assert code==200,(issuer,verifier,code,body)
            # Replay revokes the newly rotated session, even across implementations.
            assert request(urls[verifier],'POST','/v1/auth/refresh',{'refresh_token':pair['refresh_token']})[0]==401
            assert request(urls[issuer],'POST','/v1/auth/refresh',{'refresh_token':body['refresh_token']})[0]==401
            checks+=1
    assert AwsDouble.jobs and all(set(job)=={'upload_id','owner_id','original_key'} for job in AwsDouble.jobs)
    print(f'PASS: matching responses; {checks} cross-backend token/rotation/replay combinations; queue payloads match',flush=True)
    server.shutdown()

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--backends',nargs='+',choices=['rust','spring','dotnet'],default=['rust','spring','dotnet'])
    args=parser.parse_args()
    try: run(args.backends)
    finally:
        for process in reversed(PROCESSES):
            process.terminate()
            try: process.wait(timeout=10)
            except subprocess.TimeoutExpired: process.kill();process.wait()
        for log in LOGS: log.close()
