import { Application } from './lib/application';

const start = async () => {
  const PORT = Number(process.env.PORT) || 8080;
  const HOST = process.env.HOST !== '' && process.env.HOST ? String(process.env.HOST) : "0.0.0.0";

  const app = Application.getInstance();
  app.listen(PORT, HOST, () => { console.log(`Server started on http://${HOST}:${PORT}`); });
};

const main = () => {
  start().then(() => true)
    .catch((err) => {
      console.log(err);
      throw err;
    });
};

main();