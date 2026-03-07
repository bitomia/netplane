import netplaneLogoDark from "../assets/netplane_dark.svg";
import netplaneLogoLight from "../assets/netplane_light.svg";

export default function Logo() {
  return (
    <div className="flex justify-center max-w-50">
      <img
        src={netplaneLogoLight}
        className="
            logo-light h-20
            duration-700 hover:drop-shadow-[0_0_2em_rgba(36,200,219,0.8)]"
        alt="Netplane"
      />
      <img
        src={netplaneLogoDark}
        className="
            logo-dark h-20
            duration-700 hover:drop-shadow-[0_0_2em_rgba(36,200,219,0.8)]"
        alt="Netplane"
      />
    </div>
  );
}
