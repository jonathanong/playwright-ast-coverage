import express from "express";
import { createApp } from "@jongleberry/api-server";
import pathMatch from "koa-path-match";
import Router from "@koa/router";
import { Hono } from "hono";

const ignored = undefined;
const path = "/dynamic";
const expressApp = express();
const api = createApp();
const route = pathMatch();
const router = new Router();
const hono = new Hono();
const child = new Hono();
const shared = express.Router();
const v1 = express.Router();
const v2 = express.Router();

export const exported = new Router();

expressApp.get(["/array/:id", "/array/:id/edit"], () => {});
expressApp.get(path, () => {});
// Deliberately lacks a leading slash to ensure non-Koa named routes are ignored.
expressApp.all("not-a-route", () => {});
const bookRoute = expressApp.route("/books/:id");
bookRoute.get(() => {});
api.route("/api-server/:id").get(() => {});
route("/matched/:id").delete(() => {});
router.prefix("/v1");
router.get("/koa/:id", () => {});
hono.route("/child", child);
child.get("/hono-child/:id", () => {});
(hono).post("/paren/:id", () => {});
expressApp.use("/v1", v1);
expressApp.use("/v2", v2);
v1.use("/shared", shared);
v2.use("/shared", shared);
shared.get("/status", () => {});

void ignored;
