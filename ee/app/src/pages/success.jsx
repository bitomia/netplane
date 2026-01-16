

export function Success( { isLogged } ) {

    if(!isLogged) {
        return <Navigate to="/" />;
    }

    return (
        <p className='mt-6 text-base sm:text-lg text-center px-4 max-w-md text-green-500'>
        EXITO
        </p>
    )
}
    
    