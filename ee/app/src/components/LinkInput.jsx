import { Input } from "@/components/ui/input";

export default function LinkInput({ id, onChange, placeholder }) {
  return (
    <Input
      id={id}
      placeholder={placeholder}
      onChange={onChange}
      className="
            w-full flex-1 rounded-lg border border-transparent px-4 py-3 h-auto text-base font-medium 
            bg-white dark:bg-neutral-800 text-neutral-900 dark:text-white shadow-md transition-colors 
            duration-250 outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-white"
    />
  );
}
