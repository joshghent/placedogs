import express, { Request, Response } from 'express';
import asyncHandler from 'express-async-handler';
import appRoot from 'app-root-path';
import Image from '../lib/resize';
import path from 'path';

const router = express.Router();

router.get('/', (req, res) => {
  res.sendFile(path.join(`${appRoot}`, 'build', 'index.html'));
});

const randomNumber = (min: number, max: number) => {
  const r = Math.random() * (max - min) + min;
  return Math.floor(r);
};

router.get('/:width/:height', asyncHandler(async (req, res) => {
  try {
    const number = randomNumber(1, 9);
    const randomImagePath = `${number}.jpeg`;
    console.log(`Got random image ${randomImagePath}`);
    res.type('image/jpeg');
    if ((req.params.width && isNaN(Number(req.params.width))) || (req.params.height && isNaN(Number(req.params.height)))) {
      console.log(`Invalid request ${req.path}`);
      return res.status(400).json({ "message": "Please provide a valid width and height" });
    }
    console.log(`Got image path '${appRoot}/server/images/${randomImagePath}'`);
    const resizeTimer = new Date();
    const response = Image.resize(`${appRoot}/server/images/${randomImagePath}`, 'jpeg', Number(req.params.width), Number(req.params.height));
    console.log(`Completed image resize in ${(new Date().getTime() - resizeTimer.getTime())} ms`);
    const cachePath = `${appRoot}/.cache/${number}/${req.params.width}/${req.params.height}/${new Date().getTime()}.jpeg`;
    await Image.save(cachePath, response);
    return res.sendFile(cachePath);
  } catch (err) {
    console.error(err);
    throw err;
  }
}));

router.get('/health', (req: Request, res: Response) => {
  return res.json({ message: 'OK' });
});

export default router;
