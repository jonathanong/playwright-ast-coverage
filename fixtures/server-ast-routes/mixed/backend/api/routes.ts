import express from "express";
import { createApp } from "@jongleberry/api-server";
import pathMatch from "koa-path-match";
import Router from "@koa/router";
import { Hono } from "hono";

const { ignored } = {};
const path = "/dynamic";
const expressApp = express();
const api = createApp();
const route = pathMatch();
const router = new Router();
const hono = new Hono();
const child = new Hono();

export const exported = new Router();

expressApp.get(["/array/:id", "/array/:id/edit"], () => {});
expressApp.get(path, () => {});
api.route("/api-server/:id").get(() => {});
route("/matched/:id").delete(() => {});
router.prefix("/v1");
router.get("/koa/:id", () => {});
hono.route("/child", child);
child.get("/hono-child/:id", () => {});
(hono).post("/paren/:id", () => {});

void ignored;
