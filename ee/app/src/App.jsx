import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import netplaneLogoLight from "./assets/netplane_light.svg";
import netplaneLogoDark from "./assets/netplane_dark.svg";
import "./App.css";
import "@radix-ui/themes/styles.css";

function App() {
  const [clientMsg, setClient] = useState("");
  const [server, setServer] = useState("");
  const [auth, setAuth] = useState("");
  const [transport, setTransport] = useState("");

  async function client() {
    try {
      const c = await invoke("client", { server, auth, transport });
      setClient(c);
    } catch (error) {
      console.error("Error al crear client:", error);
    }



    //setClient(await invoke("client", { server, auth, transport }));
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
        Software Defined Network
      </h1>

      {/* Form Section */}
      <form
        className="w-full max-w-xl flex flex-col gap-7 sm:gap-7 justify-center items-stretch sm:items-center px-4"
        onSubmit={(e) => {
          e.preventDefault();
          client();
        }}
      >
        <input
          id="server-input"
          onChange={(e) => setServer(e.currentTarget.value)}
          placeholder="Enter server name..."
          className="w-full flex-1 rounded-lg border border-transparent px-4 py-3 text-base font-medium bg-white dark:bg-neutral-800 text-neutral-900 dark:text-white shadow-md transition-colors duration-250 outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-white"
          required
        />
        <input
          id="link-input"
          onChange={(e) => setAuth(e.currentTarget.value)}
          placeholder="Enter auth code..."
          className="w-full flex-1 rounded-lg border border-transparent px-4 py-3 text-base font-medium bg-white dark:bg-neutral-800 text-neutral-900 dark:text-white shadow-md transition-colors duration-250 outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-white" 
        />
        <span className="text-center">Select transport mode</span>
        <select
          id="transport-select"
          value={transport}
          onChange={(e) => setTransport(e.currentTarget.value)}
          className="w-2/5 mx-auto rounded-lg border border-transparent px-4 py-3 text-base font-medium bg-white dark:bg-neutral-800 text-neutral-900 dark:text-white shadow-md transition-colors duration-250 outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-white"
        >
          <option value="" selected disabled hidden>Transport mode</option>
          <option value="udp">UDP</option>
          <option value="websocket">WebSocket</option>
        </select>
        <button
          type="submit"
          className="w-1/3 mx-auto rounded-lg border border-transparent px-6 py-3 text-base font-medium bg-neutral-800 text-neutral-900 text-white shadow-md transition-all duration-250 cursor-pointer hover:border-blue-600 active:border-blue-600 active:bg-neutral-200 dark:active:bg-neutral-700 dark:hover:border-white dark:active:border-white outline-none"
        >
          Start now
        </button>
      </form>

      {/* Greeting Message */}
      {clientMsg && (
        <p className="mt-6 text-base sm:text-lg text-center px-4 max-w-md">
          {clientMsg}
        </p>
      )}
    </main>
  );
}

export default App;
