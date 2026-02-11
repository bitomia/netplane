export default function Title({ children }) {
    return (
        <h1 className="text-2xl sm:text-3xl md:text-4xl font-semibold text-center mb-6 sm:mb-8">
            {children}
        </h1>
    );
}