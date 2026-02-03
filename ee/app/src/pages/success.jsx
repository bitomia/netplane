import { useEffect, useState } from "react";
import { Navigate, useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export function Success( { isLogged, setIsLogged } ) {
    const [errorMsg, setErrorMsg] = useState(null);
    const [disconnectMsg, setDisconnectMsg] = useState("Disconnect");
    const [disableButton, setDisableButton] = useState(false);
    const navigate = useNavigate();

    useEffect(() => {
        const disconnecting_unlistener = listen('disconnecting', () => {
            setErrorMsg(null);
            setDisableButton(true);
            setDisconnectMsg("Disconnecting...");
            console.log("Disconnecting");
        });

        const disconnected_unlistener = listen('disconnected', () => {
            setIsLogged(false);
            navigate("/");
            console.log("Disconnected");
        });

        const disconnect_error_unlistener = listen('disconnect_error', (error) => {
            setErrorMsg(error.payload);
            console.error("Couldn't disconnect: ", error.payload);
            setDisconnectMsg("Disconnect");
            setDisableButton(false);
        });

        return () => {
            disconnected_unlistener.then(cleanup => cleanup());
            disconnect_error_unlistener.then(cleanup => cleanup());
            disconnecting_unlistener.then(cleanup => cleanup());
        };
    }, [navigate]);
    
    async function stop_update() {
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
            onClick={stop_update}
            >
            {disconnectMsg}
            </button>
            {/* Error Message */}
            {errorMsg && (
                <p className="mt-6 text-base sm:text-lg text-center px-4 max-w-md text-red-600">
                {errorMsg}
                </p>
            )}
        </main>
    )
}