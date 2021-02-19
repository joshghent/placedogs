import express, { Request, Response } from 'express';
import asyncHandler from 'express-async-handler';
import appRoot from 'app-root-path';
import Image from '../lib/resize';

const router = express.Router();

const randomNumber = (min: number, max: number) => {
  const r = Math.random() * (max - min) + min;
  return Math.floor(r);
};

router.get('/g/:width/:height', asyncHandler(() => {

}));

router.get('/:width/:height', asyncHandler(async (req, res) => {
  const randomImagePath = `${randomNumber(1, 9)}.jpeg`;
  res.type('image/jpeg');
  await (await Image.resize(`${appRoot}/server/images/${randomImagePath}`)).pipe(res);
}));

router.get('/health', (req: Request, res: Response) => {
  return res.json({ message: 'OK' });
});

export default router;;
