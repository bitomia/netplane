function ErrorMessage ({ message }) {
    return message &&
        <p className="mt-6 text-base sm:text-lg text-center px-4 max-w-md text-red-600">
          {message}
        </p>
    ;
}

export default ErrorMessage;