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

  const onDeleteClient = (clientId, authLink) => {
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
      <div className="grid grid-cols-4 bg-white rounded-lg px-6 py-8 w-full">
        <div>
          <img className="max-w-40" src={logo} />
          <p className="text-slate-800">Software Defined Network</p>
        </div>
        <div className="col-span-3 pl-20">
          <div className="flex flex-row justify-between items-center pb-5">
            <h3 className="text-base text-slate-950 font-medium tracking-tight text-xl">
              Network clients
            </h3>
            <button
              className="border-1 border-slate-500 hover:bg-slate-200 px-7 py-2 text-nowrap flex cursor-pointer text-slate-950"
              onClick={() => setNewClient(true)}
            >
              {" "}
              + New Client
            </button>
          </div>
          <div className="flex flex-col w-full">
            {clients &&
              clients.map((c) => (
                <div
                  className="flex flex-row w-full mb-5 bg-slate-100 hover:bg-slate-300 hover:cursor-pointer  px-4 py-3 rounded"
                  key={c.id}
                  onClick={() => setDeleteClient([c.id, c.auth_link_id])}
                >
                  <div className="w-full text-slate-700">
                    <div className="flex flex-row">
                      <div className="flex flex-col w-full">
                        <span className="font-bold text-xs">SDN IP</span>
                        {c.sdn_client_ip}
                      </div>
                      <div className="flex flex-col w-full">
                        <span className="font-bold text-xs">NETMASK</span>
                        {c.netmask}
                      </div>
                      <div className="flex flex-col w-full">
                        <span className="font-bold text-xs">Authed</span>
                        {c.used ? "✅" : "⚠️"}
                      </div>
                    </div>
                    <div className="text-[10px] pb-1">{c.id}</div>
                  </div>
                  <div className="flex items-center text-slate-900">...</div>
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
          client={deleteClient[0]}
          authLink={deleteClient[1]}
          setDeleteClient={setDeleteClient}
          onDeleteClient={onDeleteClient}
        />
      )}
    </main>
  );
}
