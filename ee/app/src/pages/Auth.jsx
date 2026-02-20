import "@radix-ui/themes/styles.css";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";

import ErrorAlert from "../components/ErrorAlert.jsx";
import LinkInput from "../components/LinkInput.jsx";
import Logo from "../components/Logo.jsx";
import Title from "../components/Title.jsx";
import ToggleTransport from "../components/ToggleTransport.jsx";

export function Auth({ setIsLogged }) {
  const { t } = useTranslation(["linkPage", "languages"]);
  const [errorMsg, setErrorMsg] = useState(null);
  const [server, setServer] = useState("");
  const [auth, setAuth] = useState("");
  const [transport, setTransport] = useState("");
  const [startMsg, setStartMsg] = useState(t("linkPage:start"));
  const [disableButton, setDisableButton] = useState(false);
  const navigate = useNavigate();

  useEffect(() => {
    const start = t("linkPage:start");
    const starting = t("linkPage:starting");
    let errorTranslation = "";

    const connectingUnlistener = listen("connecting", () => {
      setErrorMsg(null);
      setDisableButton(true);
      setStartMsg(starting);
      console.log("Starting");
    });

    const connectedUnlistener = listen("connected", () => {
      setIsLogged(true);
      navigate("/success");
      console.log("Authed");
    });

    const connectErrorUnlistener = listen("connect_error", (error) => {
      errorTranslation = "native:" + error.payload;
      setErrorMsg(t(errorTranslation));
      console.error("Couldn't link client: ", errorTranslation);
      setStartMsg(start);
      setDisableButton(false);
    });

    return () => {
      connectingUnlistener.then((cleanup) => cleanup());
      connectedUnlistener.then((cleanup) => cleanup());
      connectErrorUnlistener.then((cleanup) => cleanup());
    };
  }, [navigate]);

  async function client() {
    await invoke("client", { server, auth, transport });
  }

  return (
    <main
      className="
      min-h-screen flex flex-col items-center justify-center
      px-4 sm:px-6 py-8 bg-neutral-100 dark:bg-neutral-900
      text-neutral-900 dark:text-neutral-100"
    >
      <Logo />
      <Title>{t("linkPage:title")}</Title>

      {/* Form Section */}
      <form
        className="w-full max-w-xl flex flex-col gap-7 sm:gap-7 justify-center items-stretch sm:items-center px-4"
        onSubmit={(e) => {
          e.preventDefault();
          client();
        }}
      >
        <LinkInput
          id="server-input"
          onChange={(s) => setServer(s.currentTarget.value)}
          placeholder={t("linkPage:serverPlaceholder")}
        />
        <LinkInput
          id="link-input"
          onChange={(a) => setAuth(a.currentTarget.value)}
          placeholder={t("linkPage:linkPlaceholder")}
        />
        <ToggleTransport
          label={t("linkPage:transportMenu")}
          onValueChange={setTransport}
        />

        <button
          type="submit"
          disabled={disableButton}
          className="
            w-1/3 mx-auto rounded-lg px-4 py-3 text-base font-medium
            bg-neutral-800 text-white shadow-md
            transition-all duration-250 cursor-pointer
            hover:border-blue-600 active:border-blue-600 active:bg-neutral-200
            dark:active:bg-neutral-700 dark:hover:border-white dark:active:border-white
            outline-none"
        >
          {startMsg}
        </button>
      </form>

      {errorMsg && <ErrorAlert>{errorMsg}</ErrorAlert>}
    </main>
  );
}
