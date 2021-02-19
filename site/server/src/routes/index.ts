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

router.get('/g/:width/:height', asyncHandler(() => {

}));

router.get('/:width/:height', asyncHandler(async (req, res) => {
  const randomImagePath = `${randomNumber(1, 9)}.jpeg`;
  res.type('image/jpeg');
  if ((req.params.width && isNaN(Number(req.params.width))) || (req.params.height && isNaN(Number(req.params.height)))) {
    return res.status(400).json({ "message": "Please provide a valid width and height" });
  }
  await (await Image.resize(`${appRoot}/server/images/${randomImagePath}`, 'jpeg', Number(req.params.width), Number(req.params.height))).pipe(res);
}));

router.get('/health', (req: Request, res: Response) => {
  return res.json({ message: 'OK' });
});

export default router;
