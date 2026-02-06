import * as ToggleGroup from "@radix-ui/react-toggle-group";

function ToggleTransportItem({ id, children }) {
    return (
        <ToggleGroup.Item
            value={id}
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
            {children}
        </ToggleGroup.Item>
    );
}

export default function ToggleTransport({ value, onValueChange }) {
    return(
        <>
            <span className="text-center">Select transport mode</span>
            <ToggleGroup.Root
                type="single"
                value={value}
                onValueChange={onValueChange}
                className="
                h-9 place-items-center
                w-2/3 mx-auto flex rounded-full
                bg-white dark:bg-neutral-800
                shadow-md p-1
                focus-within:ring-2 focus-within:ring-blue-500
                dark:focus-within:ring-white"
            >
                <ToggleTransportItem id="udp">UDP</ToggleTransportItem>
                <ToggleTransportItem id="websocket">WebSocket</ToggleTransportItem>

            </ToggleGroup.Root>
        </>
    );
}