FROM node:15
LABEL maintainer "Josh Ghent <me@joshghent.com>"

WORKDIR /app
COPY . /app
# This is always 1 more than the number of images in the folder
ENV IMAGE_COUNT=11
RUN npm ci && npm run build && npm run build:server
EXPOSE 8033
CMD ["node", "./server/dist/server.js"]
