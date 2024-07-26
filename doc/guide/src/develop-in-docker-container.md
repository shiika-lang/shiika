# Develop in Docker Container

You can develop Shiika in a Docker container without configuring your local environment.

## Install Docker Compose command

At first, you need to install Docker Compose.
The recommended way is to install Docker Desktop from the official site: https://docs.docker.com/compose/install/

Note: `docker-compose` command is included in Docker Desktop.

## build and run the container

Then, you can follow the steps below:

```
docker compose up -d
docker compose exec dev bash
```

After running the above commands, you will be in the container.

`docker compose` command may be not found on your machine. Then you can use `docker-compose` command instead.
```
docker-compose up -d
docker-compose exec dev bash
```

## Run setup script

After that, run the setup script:
```
chmod +x ./setup.sh
./setup.sh
```

This script installs the necessary tools and libraries.

You've done the setup now.
