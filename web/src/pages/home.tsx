import { useState, useCallback } from "react";
import { data } from "react-router";

import logo from "../assets/reticula.svg";
import DeleteClientDialog from "./DeleteClientDialog";
import NewClientDialog from "./NewClientDialog";

export default function () {
  const [deleteClient, setDeleteClient] = useState(null);
  const [newClient, setNewClient] = useState(false);
  const [clients, setClients] = useState([]);

  useState(() => {
    fetch(`/api/clients`).then(async (res) => {
      setClients(await res.json());
    });
  }, []);

  const onDeleteClient = (clientId) => {
    fetch(`/api/clients`, {
      method: "DELETE",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ id: clientId }),
    }).then(async (res) => {
      const data = await res.json();
      setClients(data);
      setDeleteClient(false);
    });
  };

  return (
    <main className="flex items-center justify-center pt-16 pb-4 max-w-4xl w-full mx-auto">
      <div className="grid grid-cols-4 bg-gray-800 rounded-lg px-6 py-8 w-full">
        <div>
          <img className="max-w-40" src={logo} />
          <p className="text-xs text-gray-300">Software Defined Network</p>
        </div>
        <div className="col-span-3 pl-20">
          <div className="flex flex-row justify-between items-center pb-5">
            <h3 className="text-base font-medium tracking-tight text-xl">
              Network clients
            </h3>
            <button
              className="border-1 border-slate-700 hover:bg-slate-700 px-7 py-2 rounded-md text-nowrap flex cursor-pointer"
              onClick={() => setNewClient(true)}
            >
              + New Client
            </button>
          </div>
          <div className="flex flex-col w-full">
            {clients &&
              clients.map((c) => (
                <div
                  className="flex flex-row w-full mb-5 bg-gray-700 hover:bg-gray-600 hover:cursor-pointer  px-4 py-3 rounded"
                  key={c.id}
                  onClick={() => setDeleteClient(c.id)}
                >
                  <div className="w-full">
                    <div className="flex flex-row">
                      <div className="flex flex-col w-full">
                        <span className="font-bold text-xs text-gray-400">
                          SDN IP
                        </span>
                        {c.sdn_client_ip}
                      </div>
                      <div className="flex flex-col w-full">
                        <span className="font-bold text-xs text-gray-400">
                          NETMASK
                        </span>
                        {c.netmask}
                      </div>
                    </div>
                    <div className="text-[10px] pb-1 text-gray-500">{c.id}</div>
                  </div>
                  <div className="flex items-center ">...</div>
                </div>
              ))}
          </div>
        </div>
      </div>
      {newClient && (
        <NewClientDialog
          clients={clients}
          setNewClient={setNewClient}
          setClients={setClients}
        />
      )}
      {deleteClient && (
        <DeleteClientDialog
          client={deleteClient}
          setDeleteClient={setDeleteClient}
          onDeleteClient={onDeleteClient}
        />
      )}
    </main>
  );
}
