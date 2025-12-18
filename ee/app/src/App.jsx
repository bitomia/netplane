import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import netplaneLogoLight from "./assets/netplane_light.svg";
import netplaneLogoDark from "./assets/netplane_dark.svg";
import "./App.css";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <main className="min-h-screen flex flex-col items-center justify-center px-4 sm:px-6 py-8 bg-neutral-100 dark:bg-neutral-900 text-neutral-900 dark:text-neutral-100">
      {/* Logo Section */}
      <div className="flex justify-center mb-6 sm:mb-8">
        <img
          src={netplaneLogoLight}
          className="logo-light h-20 sm:h-24 md:h-28 lg:h-32 p-4 transition-all duration-700 hover:drop-shadow-[0_0_2em_rgba(36,200,219,0.8)]"
          alt="Netplane"
        />
        <img
          src={netplaneLogoDark}
          className="logo-dark h-20 sm:h-24 md:h-28 lg:h-32 p-4 transition-all duration-700 hover:drop-shadow-[0_0_2em_rgba(36,200,219,0.8)]"
          alt="Netplane"
        />
      </div>

      {/* Title */}
      <h1 className="text-2xl sm:text-3xl md:text-4xl font-semibold text-center mb-6 sm:mb-8">
        Welcome to Netplane
      </h1>

      {/* Form Section */}
      <form
        className="w-full max-w-md flex flex-col sm:flex-row gap-3 sm:gap-2 justify-center items-stretch sm:items-center px-4"
        onSubmit={(e) => {
          e.preventDefault();
          greet();
        }}
      >
        <input
          id="greet-input"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a name..."
          className="flex-1 rounded-lg border border-transparent px-4 py-3 text-base font-medium bg-white dark:bg-neutral-800 text-neutral-900 dark:text-white shadow-md transition-colors duration-250 outline-none focus:ring-2 focus:ring-blue-500"
        />
        <button
          type="submit"
          className="rounded-lg border border-transparent px-6 py-3 text-base font-medium bg-white dark:bg-neutral-800 text-neutral-900 dark:text-white shadow-md transition-all duration-250 cursor-pointer hover:border-blue-600 active:border-blue-600 active:bg-neutral-200 dark:active:bg-neutral-700 outline-none"
        >
          Greet
        </button>
      </form>

      {/* Greeting Message */}
      {greetMsg && (
        <p className="mt-6 text-base sm:text-lg text-center px-4 max-w-md">
          {greetMsg}
        </p>
      )}
    </main>
  );
}

export default App;
