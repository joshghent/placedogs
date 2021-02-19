import express from 'express';
import bodyParser from 'body-parser';
import helmet from 'helmet';
import { middleware as httpContextMiddleware } from "express-http-context";
import router from '../routes';

export class Application {
  private static instance: express.Application;
  public static getInstance(): express.Application {
    if (this.instance) {
      return this.instance;
    }

    this.instance = express();

    this.instance.use(bodyParser.json());
    this.instance.use(bodyParser.urlencoded({ extended: true }));
    this.instance.use(httpContextMiddleware);
    this.instance.use(helmet());

    this.instance.use(router);

    this.instance.use((req: express.Request, res: express.Response) => {
      if (!res.headersSent) res.status(404);
    });

    this.instance.use((err: Error, req: express.Request, res: express.Response, next: express.NextFunction) => {
      if (process.env.NODE_ENV === "dev") return res.status(500).jsonp({ error: { message: err.message, stack: err.stack } });
      return res.status(500).jsonp({ error: { message: err.message } });
    });

    return this.instance;
  }
}

export default Application;
