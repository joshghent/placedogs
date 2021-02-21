import fs from 'fs';
import sharp from 'sharp';

export class Image {
  public static resize(path: string, format?: string, width?: number, height?: number) {
    const readStream = fs.createReadStream(path);
    let transform = sharp();
    if (width || height) {
      transform = transform.resize(width, height);
    }
    return readStream.pipe(transform);
  };
}

export default Image;
