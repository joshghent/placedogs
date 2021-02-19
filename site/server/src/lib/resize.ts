import fs from 'fs';
export class Image {
  public static async resize(path: string, format?: string, width?: number, height?: number) {
    const readStream = fs.createReadStream(path);
    return readStream;
  };
}

export default Image;
