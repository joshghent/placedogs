"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.Image = void 0;
const fs_1 = __importDefault(require("fs"));
class Image {
    static async resize(path, format, width, height) {
        const readStream = fs_1.default.createReadStream(path);
        return readStream;
    }
    ;
}
exports.Image = Image;
exports.default = Image;
