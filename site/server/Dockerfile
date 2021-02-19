FROM node:12
LABEL maintainer "Josh Ghent <me@joshghent.com>"

WORKDIR /app
COPY . /app
RUN npm ci && npm run build
EXPOSE 8080
CMD ["node", "dist/server.js"]
