"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const express_1 = __importDefault(require("express"));
const express_async_handler_1 = __importDefault(require("express-async-handler"));
const app_root_path_1 = __importDefault(require("app-root-path"));
const resize_1 = __importDefault(require("../lib/resize"));
const router = express_1.default.Router();
const randomNumber = (min, max) => {
    const r = Math.random() * (max - min) + min;
    return Math.floor(r);
};
router.get('/g/:width/:height', express_async_handler_1.default(() => {
}));
router.get('/:width/:height', express_async_handler_1.default(async (req, res) => {
    const randomImagePath = `${randomNumber(1, 9)}.jpeg`;
    res.type('image/jpeg');
    if ((req.params.width && isNaN(Number(req.params.width))) || (req.params.height && isNaN(Number(req.params.height)))) {
        return res.status(400).json({ "message": "Please provide a valid width and height" });
    }
    await (await resize_1.default.resize(`${app_root_path_1.default}/server/images/${randomImagePath}`, 'jpeg', Number(req.params.width), Number(req.params.height))).pipe(res);
}));
router.get('/health', (req, res) => {
    return res.json({ message: 'OK' });
});
exports.default = router;
;
