FROM node:20
LABEL maintainer="Josh Ghent <me@joshghent.com>"

# Install curl for healthcheck and Sharp dependencies
RUN apt-get update && \
    apt-get install -y curl \
    libvips-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . /app
# This is always 1 more than the number of images in the folder
ENV IMAGE_COUNT=32
RUN yarn install --frozen-lockfile --prefer-offline
RUN yarn run build
RUN yarn run build:server
EXPOSE 8033
CMD ["node", "./server/dist/server.js"]
