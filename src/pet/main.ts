import { mount } from "svelte";
import { forwardErrors } from "../lib/logging";
import Pet from "./Pet.svelte";

forwardErrors("pet");
mount(Pet, { target: document.getElementById("root")! });
