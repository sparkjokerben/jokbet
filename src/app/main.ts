import { mount } from "svelte";
import { forwardErrors } from "../lib/logging";
import App from "./App.svelte";
import "./theme.css";

forwardErrors(new URLSearchParams(location.search).get("view") ?? "app");
mount(App, { target: document.getElementById("root")! });
