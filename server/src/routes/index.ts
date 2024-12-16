import express, { Request, Response, NextFunction } from "express";
import asyncHandler from "express-async-handler";
import appRoot from "app-root-path";
import rateLimit from "express-rate-limit";
import Image from "../lib/image";
import path from "path";
import fs from "fs";

const router = express.Router();

// Get the number of files in the images folder
const imageCount = fs.readdirSync(`${appRoot}/server/images`).length;

const limiter = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 100, // Limit each IP to 100 requests per windowMs
  message: 'Too many requests from this IP, please try again later'
});

router.get("/", limiter, (req: Request, res: Response) => {
  res.sendFile(path.join(`${appRoot}`, "build", "index.html"));
});

const randomNumber = (min: number, max: number) => {
  const r = Math.random() * (max - min) + min;
  return Math.floor(r);
};

router.get(
  "/:width/:height",
  asyncHandler(async (req: Request, res: Response, next: NextFunction) => {
    try {
      console.log(
        `Got request for width: ${req.params.width} and height: ${req.params.height}`
      );
      const regex = /^\d+$/;
      if (
        !req.params.width ||
        req.params.width === "" ||
        !regex.test(req.params.width)
      ) {
        console.log(`Invalid request ${req.path}`);
        res
          .status(400)
          .json({ message: "Please provide a valid width and height" });
        return;
      }

      if (
        !req.params.height ||
        req.params.height === "" ||
        !regex.test(req.params.height)
      ) {
        console.log(`Invalid request ${req.path}`);
        res
          .status(400)
          .json({ message: "Please provide a valid width and height" });
        return;
      }

      if (Number(req.params.width) > 3048 || Number(req.params.height) > 3048) {
        console.log(`Invalid request ${req.path}`);
        res.status(400).json({
          message: "Please provide a width and height below 3048 pixels",
        });
        return;
      }

      console.log(`Total images in store: ${imageCount}`);
      const maxImageId = imageCount ?? 10;
      const number = randomNumber(1, Number(maxImageId));
      const randomImagePath = `${number}.jpeg`;
      console.log(`Got random image ${randomImagePath}`);
      res.type("image/jpeg");
      console.log(
        `Got image path '${appRoot}/server/images/${randomImagePath}'`
      );

      // Fetch image from the cache if present
      const cacheFolder = `${appRoot}/.cache/${number}/${req.params.width}/${req.params.height}`;
      const file = await Image.getImageFromCache(cacheFolder);
      console.log(file);
      if (file !== "") {
        return res.sendFile(file);
      }

      const resizeTimer = new Date();
      const response = Image.resize(
        `${appRoot}/server/images/${randomImagePath}`,
        "jpeg",
        Number(req.params.width),
        Number(req.params.height)
      );
      console.log(
        `Completed image resize in ${
          new Date().getTime() - resizeTimer.getTime()
        } ms`
      );
      const cachePath = `${cacheFolder}/${new Date().getTime()}.jpeg`;
      await Image.save(cachePath, response);
      console.log("Successfully cached image. Returning file to request now");
      res.sendFile(cachePath);
      return;
    } catch (err) {
      next(err);
    }
  })
);

router.get("/health", (req: Request, res: Response) => {
  const used = process.memoryUsage();
  res.json({
    status: 'ok',
    uptime: process.uptime(),
    memory: {
      heapUsed: Math.round(used.heapUsed / 1024 / 1024) + 'MB',
      heapTotal: Math.round(used.heapTotal / 1024 / 1024) + 'MB',
      rss: Math.round(used.rss / 1024 / 1024) + 'MB',
    }
  });
});

export default router;
