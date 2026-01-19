import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import netplaneLogoLight from "../assets/netplane_light.svg";
import netplaneLogoDark from "../assets/netplane_dark.svg";
import "@radix-ui/themes/styles.css";
import * as ToggleGroup from "@radix-ui/react-toggle-group";
import { useNavigate } from "react-router-dom";

export function Auth({ setisLogged }) {
  const [errorMsg, setErrorMsg] = useState(null);
  const [server, setServer] = useState("");
  const [auth, setAuth] = useState("");
  const [transport, setTransport] = useState("");
  const navigate = useNavigate();

  async function client() {
    try {
      const c = await invoke("client", { server, auth, transport });

      setisLogged(true);
      navigate("/success");
    } catch (error) {
      setErrorMsg(error);

      console.error("Error al crear client:", error);
    }
  }

  return (
    <main
      className="
      min-h-screen flex flex-col items-center justify-center
      px-4 sm:px-6 py-8 bg-neutral-100 dark:bg-neutral-900
      text-neutral-900 dark:text-neutral-100"
    >
      {/* Logo Section */}
      <div className="flex justify-center mb-6 sm:mb-8">
        <img
          src={netplaneLogoLight}
          className="
          logo-light h-20 sm:h-24 md:h-28 lg:h-32 p-4 transition-all
          duration-700 hover:drop-shadow-[0_0_2em_rgba(36,200,219,0.8)]"
          alt="Netplane"
        />
        <img
          src={netplaneLogoDark}
          className="
          logo-dark h-20 sm:h-24 md:h-28 lg:h-32 p-4 transition-all
          duration-700 hover:drop-shadow-[0_0_2em_rgba(36,200,219,0.8)]"
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
          onChange={(s) => setServer(s.currentTarget.value)}
          placeholder="Enter server name..."
          className="
          w-full flex-1 rounded-lg border border-transparent px-4 py-3 text-base font-medium 
          bg-white dark:bg-neutral-800 text-neutral-900 dark:text-white shadow-md transition-colors 
          duration-250 outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-white"
        />
        <input
          id="link-input"
          onChange={(a) => setAuth(a.currentTarget.value)}
          placeholder="Enter auth code..."
          className="
          w-full flex-1 rounded-lg border border-transparent px-4 py-3 text-base font-medium 
          bg-white dark:bg-neutral-800 text-neutral-900 dark:text-white shadow-md transition-colors 
          duration-250 outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-white"
        />
        <span className="text-center">Select transport mode</span>
        <ToggleGroup.Root
          type="single"
          value={transport}
          onValueChange={setTransport}
          className="
            h-9 place-items-center
            w-2/3 mx-auto flex rounded-full
            bg-white dark:bg-neutral-800
            shadow-md p-1
            focus-within:ring-2 focus-within:ring-blue-500
            dark:focus-within:ring-white"
        >
          <ToggleGroup.Item
            value="udp"
            className="
              items-center justify-center
              flex-1 px-4 py-1 text-base font-medium
              rounded-full transition-colors duration-200
              text-neutral-900 dark:text-white
              data-[state=on]:bg-blue-600
              data-[state=on]:text-white
              data-[state=on]:hover:bg-blue-500
              hover:bg-neutral-100 dark:hover:bg-neutral-700
              dark:data-[state=on]:bg-white
              dark:data-[state=on]:text-black
              dark:data-[state=on]:hover:bg-neutral-200"
          >
            UDP
          </ToggleGroup.Item>

          <ToggleGroup.Item
            value="websocket"
            className="
              items-center justify-center
              flex-1 px-4 py-1 text-base font-medium
              rounded-full transition-colors duration-200
              text-neutral-900 dark:text-white
              data-[state=on]:bg-blue-600
              data-[state=on]:text-white
              data-[state=on]:hover:bg-blue-500
              hover:bg-neutral-100 dark:hover:bg-neutral-700
              dark:data-[state=on]:bg-white
              dark:data-[state=on]:text-black
              dark:data-[state=on]:hover:bg-neutral-200"
          >
            WebSocket
          </ToggleGroup.Item>
        </ToggleGroup.Root>
        <button
          type="submit"
          className="
            w-1/3 mx-auto rounded-lg px-4 py-3 text-base font-medium
            bg-neutral-800 text-white shadow-md
            transition-all duration-250 cursor-pointer
            hover:border-blue-600 active:border-blue-600 active:bg-neutral-200
            dark:active:bg-neutral-700 dark:hover:border-white dark:active:border-white
            outline-none"
        >
          Start now
        </button>
      </form>

      {/* Greeting Message */}
      {errorMsg && (
        <p className="mt-6 text-base sm:text-lg text-center px-4 max-w-md text-red-600">
          {errorMsg}
        </p>
      )}
    </main>
  );
}