export default function Input({ id, value, onChange, placeholder }) {
    return (
        <input
          id={id}
          value={value}
          onChange={onChange}
          placeholder={placeholder}
          className="
          w-full flex-1 rounded-lg border border-transparent px-4 py-3 text-base font-medium 
          bg-white dark:bg-neutral-800 text-neutral-900 dark:text-white shadow-md transition-colors 
          duration-250 outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-white"
        />
        
    );

}