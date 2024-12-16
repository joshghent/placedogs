import express, { NextFunction, Request, Response } from 'express';
import helmet from 'helmet';
import path from 'path';
import morgan from 'morgan';
import appRoot from 'app-root-path';
import * as httpContext from "express-http-context2";
import router from '../routes';

export class Application {
  private static instance: express.Application;
  public static getInstance(): express.Application {
    if (this.instance) {
      return this.instance;
    }

    this.instance = express();

    this.instance.use(express.static(path.join(`${appRoot}`, 'build')));
    this.instance.use(express.static(path.join(`${appRoot}`, 'server', 'images')));

    this.instance.use(express.json());
    this.instance.use(express.urlencoded({ extended: true }));
    this.instance.use(httpContext.middleware);
    this.instance.use(helmet({ contentSecurityPolicy: false }));

    this.instance.use(morgan(':method :status :url (:res[content-length] bytes) :response-time ms', {
      stream: { write: (text: string) => console.info(text.trim()) }
    }));

    this.instance.use(router);

    this.instance.use((req: Request, res: Response) => {
      if (!res.headersSent) res.status(404).send();
    });

    this.instance.use((err: Error, req: Request, res: Response, next: NextFunction) => {
      console.log(`Error when processing request (${req.path}). Error: ${JSON.stringify(err)}`);
      if (process.env.NODE_ENV === "dev") return res.status(500).jsonp({ error: { message: err.message, stack: err.stack } });
      return res.status(500).jsonp({ error: { message: err.message } });
    });

    return this.instance;
  }
}

export default Application;
