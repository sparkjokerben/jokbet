import { mount } from "svelte";
import { applyLanguage } from "../lib/language";
import { forwardErrors } from "../lib/logging";
import Pet from "./Pet.svelte";

forwardErrors("pet");
await applyLanguage();
mount(Pet, { target: document.getElementById("root")! });
