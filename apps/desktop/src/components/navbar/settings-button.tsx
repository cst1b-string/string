import SettingsGear from "@/assets/settings-gear.png";
import { Link } from "react-router-dom";

export const SettingsButton = () => (
	<Link to="/settings">
		<img src={SettingsGear} alt="Settings" width={40} height={30} />
	</Link>
);
