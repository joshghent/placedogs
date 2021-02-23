FROM node:15
LABEL maintainer "Josh Ghent <me@joshghent.com>"

WORKDIR /app
COPY . /app
RUN npm ci && npm run build && npm run build:server
EXPOSE 8033
CMD ["node", "./server/dist/server.js"]
