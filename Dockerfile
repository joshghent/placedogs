FROM node:15-alpine
LABEL maintainer "Josh Ghent <me@joshghent.com>"

ENV SHARP_VERSION 0.27.1

#Compile Vips and Sharp
RUN	apk --no-cache add libpng librsvg libgsf giflib libjpeg-turbo musl \
  && apk add vips-dev fftw-dev build-base --update-cache  --repository https://alpine.global.ssl.fastly.net/alpine/edge/testing/  --repository https://alpine.global.ssl.fastly.net/alpine/edge/main \
  && apk --no-cache add --virtual .build-dependencies g++ make python curl tar gtk-doc gobject-introspection expat-dev glib-dev libpng-dev libjpeg-turbo-dev giflib-dev librsvg-dev  \
  && su node \
  && npm install sharp@${SHARP_VERSION} --g --production --unsafe-perm \
  && chown node:node /usr/local/lib/node_modules -R \
  && apk del .build-dependencies

RUN apt-get update && apt-get install --force-yes -yy \
  libjemalloc1 \
  && rm -rf /var/lib/apt/lists/*

# Change memory allocator to avoid leaks
ENV LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libjemalloc.so.1

WORKDIR /app
COPY . /app
RUN npm ci && npm run build && npm run build:server
EXPOSE 8033
CMD ["node", "./server/dist/server.js"]
