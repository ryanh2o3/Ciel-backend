FROM mcr.microsoft.com/dotnet/sdk:8.0 AS build
WORKDIR /src
COPY Ciel.Api/Ciel.Api.csproj Ciel.Api/
RUN dotnet restore Ciel.Api/Ciel.Api.csproj
COPY Ciel.Api/ Ciel.Api/
RUN dotnet publish Ciel.Api/Ciel.Api.csproj -c Release -o /app/publish /p:UseAppHost=false

FROM mcr.microsoft.com/dotnet/aspnet:8.0 AS runtime
WORKDIR /app
ENV ASPNETCORE_URLS=http://0.0.0.0:8080
EXPOSE 8080

# curl is needed for the HEALTHCHECK probe below; install it as root before
# dropping privileges.
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/publish .

# The aspnet base image ships a non-root "app" user (UID $APP_UID); running
# as non-root matches the Rust image's hardening and container security
# best practice in general.
USER $APP_UID

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -fsS http://localhost:8080/health || exit 1

ENTRYPOINT ["dotnet", "Ciel.Api.dll"]
