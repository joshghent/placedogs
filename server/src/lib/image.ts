import fs from 'fs';
import fse from 'fs-extra';
import sharp from 'sharp';

export class Image {
  public static resize(path: string, format?: string, width?: number, height?: number) {
    try {
      const readStream = fs.createReadStream(path);
      let transform = sharp();

      readStream.on('error', (err) => {
        console.error(JSON.stringify({
          event: 'read_stream_error',
          error: err.message,
          path
        }));
      });

      transform.on('error', (err) => {
        console.error(JSON.stringify({
          event: 'transform_error',
          error: err.message,
          width,
          height
        }));
      });

      if (width || height) {
        transform = transform.resize(width, height);
      }

      return readStream.pipe(transform);
    } catch (err: any) {
      console.error(JSON.stringify({
        event: 'resize_error',
        error: err.message,
        path,
        width,
        height
      }));
      throw err;
    }
  }

  public static async save(path: string, image: sharp.Sharp) {
    try {
      console.log("Transforming image to buffer");
      const buffer = await image.toBuffer();
      console.log("Saved image to Buffer");
      const res = await fse.outputFileSync(path, buffer);
      console.log(`Saved new file ${path} to cache`);
      return res;
    } catch (err) {
      console.error(err);
    }
  }

  public static async getImageFromCache(cacheFolder: string) {
    console.log("Getting image from cache if any");

    try {
      const files = await fs.readdirSync(cacheFolder);
      let fileName = '';
      if (files.length > 0) {
        fileName = `${cacheFolder}/${files[0]}`;
      }

      return fileName;
    } catch (err) {
      console.log(`Could not read directory or it doesn't exist. Probably the latter. ${err}`);
      return '';
    }
  }
}

export default Image;
