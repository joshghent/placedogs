import { Application } from './lib/application';

const start = async () => {
  const PORT = Number(process.env.PORT) || 8033;
  const HOST = process.env.HOST !== '' && process.env.HOST ? String(process.env.HOST) : "0.0.0.0";

  const app = Application.getInstance();
  const httpServer = app.listen(PORT, HOST, () => { console.log(`Server started on http://${HOST}:${PORT}`); });

  const terminateConnection = (err: Error) => {
    console.log(`Server Terminated Error: ${JSON.stringify(err)}`);
    try {
      if (httpServer) {
        httpServer.close();
      }
    } catch (err) {
      console.error(err);
    }
  };

  process.on("exit", terminateConnection);
  process.on("unhandledRejection", terminateConnection);
  process.on("uncaughtException", terminateConnection); // unexpected crash
  process.on("SIGINT", terminateConnection); // Ctrl + C (1)
};

const main = () => {
  start().then(() => true)
    .catch((err) => {
      console.log(err);
      throw err;
    });
};

main();;;
