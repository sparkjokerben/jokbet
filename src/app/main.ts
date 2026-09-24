import { mount } from "svelte";
import { applyLanguage } from "../lib/language";
import { forwardErrors } from "../lib/logging";
import App from "./App.svelte";
import "./theme.css";

forwardErrors(new URLSearchParams(location.search).get("view") ?? "app");
await applyLanguage();
mount(App, { target: document.getElementById("root")! });
