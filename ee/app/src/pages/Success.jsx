import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "react-router-dom";

import { useTranslation } from 'react-i18next';

export function Success( { isLogged, setIsLogged } ) {
    const navigate = useNavigate();

    const {t, i18n} = useTranslation();
    
    async function stop_update() {
        await invoke("stop_update", {});
        setIsLogged(false);
        navigate("/");
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
            className="
                w-1/3 rounded-lg px-4 py-8 text-base font-medium
                bg-neutral-800 text-white shadow-md
                transition-all duration-250 cursor-pointer
                hover:border-blue-600 active:border-blue-600 active:bg-neutral-200
                dark:active:bg-neutral-700 dark:hover:border-white dark:active:border-white
                outline-none"
            onClick={stop_update}
            >
            {t('disconnect')}
            </button>
        </main>
    )
}