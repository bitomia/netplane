import { useEffect, useState } from "react";
import { Navigate, useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from 'react-i18next';

import ErrorAlert from "../components/ErrorAlert";

export function Success( { isLogged, setIsLogged } ) {
    const { t } = useTranslation();
    const [errorMsg, setErrorMsg] = useState(null);
    const [disconnectMsg, setDisconnectMsg] = useState(t('successPage:disconnect'));
    const [disableButton, setDisableButton] = useState(false);
    const navigate = useNavigate();

    useEffect(() => {
        const disconnect = t('successPage:disconnect');
        const disconnecting = t('successPage:disconnecting');
        let errorTranslation = "";

        const disconnectingUnlistener = listen('disconnecting', () => {
            setErrorMsg(null);
            setDisableButton(true);
            setDisconnectMsg(disconnecting);
            console.log("Disconnecting");
        });

        const disconnectedUnlistener = listen('disconnected', () => {
            setIsLogged(false);
            navigate("/");
            console.log("Disconnected");
        });

        const disconnectErrorUnlistener = listen('disconnect_error', (error) => {
            errorTranslation = "native:" + error.payload;
            setErrorMsg(t(errorTranslation));
            console.error("Couldn't disconnect: ", errorTranslation);
            setDisconnectMsg(disconnect);
            setDisableButton(false);
        });

        return () => {
            disconnectingUnlistener.then(cleanup => cleanup());
            disconnectedUnlistener.then(cleanup => cleanup());
            disconnectErrorUnlistener.then(cleanup => cleanup());
        };
    }, [navigate]);
    
    async function stopUpdate() {
        await invoke("stop_update", {});
    }
    
    if(!isLogged) {
        return <Navigate to="/" />;
    }

    return (
        <main className="
        min-h-screen flex flex-col items-center justify-center
        px-4 sm:px-6 py-8 bg-neutral-100 dark:bg-neutral-900
        text-neutral-900 dark:text-neutral-100
        ">
            <button
            type="submit"
            disabled = {disableButton}
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
            
            {errorMsg && <ErrorAlert>{errorMsg}</ErrorAlert>}
        </main>
    )
}