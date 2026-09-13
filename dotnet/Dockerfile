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
COPY --from=build /app/publish .
ENTRYPOINT ["dotnet", "Ciel.Api.dll"]
