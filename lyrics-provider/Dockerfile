# use the official Bun image
# see all versions at https://hub.docker.com/r/oven/bun/tags
FROM oven/bun:1.2.13 AS base
WORKDIR /usr/src/app

# copy production dependencies and source code into final image
FROM base AS release
COPY package.json bun.lockb .
RUN bun install --frozen-lockfile --production

COPY . .

# run the app
USER bun
EXPOSE 3000/tcp
ENTRYPOINT [ "bun", "run", "index.ts" ]
