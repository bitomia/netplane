import { useMemo, useState } from "react";

export default ({ setNewClient, onCreateClient }) => {
  const [sdnIP, setSDNIP] = useState("");
  const [clientKey, setClientKey] = useState("");
  const createDisabled = useMemo(
    () => sdnIP?.length == 0 || clientKey.length == 0,
    [sdnIP, clientKey],
  );

  return (
    <div
      className="relative z-10"
      aria-labelledby="modal-title"
      role="dialog"
      aria-modal="true"
    >
      <div
        className="fixed inset-0 bg-gray-500/75 transition-opacity"
        aria-hidden="true"
      ></div>

      <div className="fixed inset-0 z-10 w-screen overflow-y-auto">
        <div className="flex min-h-full items-end justify-center p-4 text-center sm:items-center sm:p-0">
          <div className="relative transform overflow-hidden rounded-lg bg-white text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-lg">
            <div className="bg-white px-4 pt-5 pb-4 sm:p-6 sm:pb-4">
              <div className="sm:flex sm:items-start">
                <div className="mt-3 text-center sm:mt-0 sm:ml-4 sm:text-left w-full">
                  <h3
                    className="text-base font-semibold text-gray-900"
                    id="modal-title"
                  >
                    Create client
                  </h3>
                  <div className="mt-2 text-sm text-gray-700 flex flex-col">
                    <label className="mt-3">Client Key</label>
                    <input
                      type="text"
                      placeholder="Signed key"
                      className="px-3 py-2 border border-slate-200 rounded invalid:border-pink-500 invalid:text-pink-600 focus:invalid:border-pink-500 focus:invalid:outline-pink-500"
                      onChange={(e) => {
                        if (e.currentTarget.validity.valid) {
                          setClientKey(e.currentTarget.value);
                        }
                      }}
                      minLength={7}
                      maxLength={15}
                      size={15}
                      pattern="^(?>(\d|[1-9]\d{2}|1\d\d|2[0-4]\d|25[0-5])\.){3}(?1)$"
                      autoComplete="off"
                    />
                    <label className="mt-3">SDN IP Address</label>
                    <input
                      type="text"
                      placeholder="xxx.xxx.xxx.xxx"
                      className="px-3 py-2 border border-slate-200 rounded invalid:border-pink-500 invalid:text-pink-600 focus:invalid:border-pink-500 focus:invalid:outline-pink-500"
                      onChange={(e) => {
                        if (e.currentTarget.validity.valid) {
                          setSDNIP(e.currentTarget.value);
                        }
                      }}
                      minLength={7}
                      maxLength={15}
                      size={15}
                      pattern="^(?>(\d|[1-9]\d{2}|1\d\d|2[0-4]\d|25[0-5])\.){3}(?1)$"
                      autoComplete="off"
                    />
                  </div>
                </div>
              </div>
            </div>
            <div className="bg-gray-50 px-4 py-3 sm:flex sm:flex-row-reverse sm:px-6">
              <button
                type="button"
                disabled={createDisabled}
                className="inline-flex w-full justify-center rounded-md bg-green-600 px-3 py-2 text-sm font-semibold text-white shadow-xs hover:bg-green-500 sm:ml-3 sm:w-auto disabled:bg-slate-200"
                onClick={() => onCreateClient(sdnIP, clientKey)}
              >
                Create
              </button>
              <button
                type="button"
                className="mt-3 inline-flex w-full justify-center rounded-md bg-white px-3 py-2 text-sm font-semibold text-gray-900 ring-1 shadow-xs ring-gray-300 ring-inset hover:bg-gray-50 sm:mt-0 sm:w-auto"
                onClick={() => setNewClient(null)}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
