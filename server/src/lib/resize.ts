import fs from 'fs';
import sharp from 'sharp';

export class Image {
  public static resize(path: string, format?: string, width?: number, height?: number) {
    try {
      const readStream = fs.createReadStream(path);
      console.log(`Created read stream to '${path}'`);
      let transform = sharp();
      console.log(`Created sharp instance`);
      if (width || height) {
        console.log(`Transforming image to h: ${height}, w: ${width}`);
        transform = transform.resize(width, height);
      }

      console.log(`Returning transform to response`);
      return readStream.pipe(transform);
    } catch (err) {
      console.error(err);
      throw err;
    }
  };
}

export default Image;
