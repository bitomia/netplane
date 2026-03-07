import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Navigate, useNavigate } from "react-router-dom";

import ErrorAlert from "../components/ErrorAlert";

export function Success({ isLogged, setIsLogged }) {
  const { t } = useTranslation();
  const [errorMsg, setErrorMsg] = useState(null);
  const [disconnectMsg, setDisconnectMsg] = useState(
    t("successPage:disconnect"),
  );
  const [disableButton, setDisableButton] = useState(false);
  const [connectionStatus, setConnectionStatus] = useState("offline");
  const timeoutRef = useRef(null);
  const navigate = useNavigate();

  useEffect(() => {
    const disconnect = t("successPage:disconnect");
    const disconnecting = t("successPage:disconnecting");
    let errorTranslation = "";

    const disconnectingUnlistener = listen("disconnecting", () => {
      setErrorMsg(null);
      setDisableButton(true);
      setDisconnectMsg(disconnecting);
      console.log("Disconnecting");
    });

    const disconnectedUnlistener = listen("disconnected", () => {
      setIsLogged(false);
      navigate("/");
      console.log("Disconnected");
    });

    const stateUpdatedUnlistener = listen("state-updated", () => {
      setConnectionStatus("online");
      clearTimeout(timeoutRef.current);
      timeoutRef.current = setTimeout(() => {
        setConnectionStatus("offline");
      }, 5000);
    });

    const disconnectErrorUnlistener = listen("disconnect_error", (error) => {
      errorTranslation = `native:${error.payload}`;
      setErrorMsg(t(errorTranslation));
      console.error("Couldn't disconnect: ", errorTranslation);
      setDisconnectMsg(disconnect);
      setDisableButton(false);
    });

    return () => {
      disconnectingUnlistener.then((cleanup) => cleanup());
      disconnectedUnlistener.then((cleanup) => cleanup());
      stateUpdatedUnlistener.then((cleanup) => cleanup());
      disconnectErrorUnlistener.then((cleanup) => cleanup());
      clearTimeout(timeoutRef.current);
    };
  }, [navigate, setIsLogged, t]);

  async function stopUpdate() {
    await invoke("stop_update", {});
  }

  if (!isLogged) {
    return <Navigate to="/" />;
  }

  return (
    <main
      className="
        min-h-screen flex flex-col items-center justify-center
        px-4 sm:px-6 py-8 bg-neutral-100 dark:bg-neutral-900
        text-neutral-900 dark:text-neutral-100
        "
    >
      <button
        type="submit"
        disabled={disableButton}
        className="
                w-1/3 rounded-lg px-4 py-8 text-base font-medium
                bg-neutral-800 text-white shadow-md
                transition-all duration-250 cursor-pointer
                hover:border-blue-600 active:border-blue-600 active:bg-neutral-200
                dark:active:bg-neutral-700 dark:hover:border-white dark:active:border-white
                outline-none"
        onClick={stopUpdate}
      >
        {disconnectMsg}
      </button>

      <p
        className={`mt-6 text-sm font-medium ${
          connectionStatus === "online"
            ? "text-green-600 dark:text-green-400"
            : "text-red-600 dark:text-red-400"
        }`}
      >
        {connectionStatus === "online"
          ? t("successPage:online")
          : t("successPage:offline")}
      </p>

      {errorMsg && <ErrorAlert>{errorMsg}</ErrorAlert>}
    </main>
  );
}
