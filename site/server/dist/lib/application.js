"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.Application = void 0;
const express_1 = __importDefault(require("express"));
const body_parser_1 = __importDefault(require("body-parser"));
const helmet_1 = __importDefault(require("helmet"));
const path_1 = __importDefault(require("path"));
const app_root_path_1 = __importDefault(require("app-root-path"));
const express_http_context_1 = require("express-http-context");
const routes_1 = __importDefault(require("../routes"));
class Application {
    static getInstance() {
        if (this.instance) {
            return this.instance;
        }
        this.instance = express_1.default();
        this.instance.use(express_1.default.static(path_1.default.join(`${app_root_path_1.default}`, 'build')));
        this.instance.use(body_parser_1.default.json());
        this.instance.use(body_parser_1.default.urlencoded({ extended: true }));
        this.instance.use(express_http_context_1.middleware);
        this.instance.use(helmet_1.default({ contentSecurityPolicy: false }));
        this.instance.use(routes_1.default);
        this.instance.use((req, res) => {
            if (!res.headersSent)
                res.status(404);
        });
        this.instance.use((err, req, res, next) => {
            if (process.env.NODE_ENV === "dev")
                return res.status(500).jsonp({ error: { message: err.message, stack: err.stack } });
            return res.status(500).jsonp({ error: { message: err.message } });
        });
        return this.instance;
    }
}
exports.Application = Application;
exports.default = Application;
