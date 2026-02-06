import netplaneLogoLight from "../assets/netplane_light.svg";
import netplaneLogoDark from "../assets/netplane_dark.svg";

export default function Logo() {
    return (
        <div className="flex justify-center mb-6 sm:mb-8">
            <img
            src={netplaneLogoLight}
            className="
            logo-light h-20 sm:h-24 md:h-28 lg:h-32 p-4 transition-all
            duration-700 hover:drop-shadow-[0_0_2em_rgba(36,200,219,0.8)]"
            alt="Netplane"
            />
            <img
            src={netplaneLogoDark}
            className="
            logo-dark h-20 sm:h-24 md:h-28 lg:h-32 p-4 transition-all
            duration-700 hover:drop-shadow-[0_0_2em_rgba(36,200,219,0.8)]"
            alt="Netplane"
            />
        </div>
    );
}