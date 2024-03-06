import Image from "next/image";
import Link from "next/link";
import { useContext } from "react";

import { LoginContext } from "../contexts/loginContext";

export const SettingsButton = () => {
	const { isLoggedIn } = useContext(LoginContext);

	if (!isLoggedIn) {
		return null;
	}
	return (
		<Link href="/settings">
			<Image src="/settings-gear.png" alt="Settings" width={40} height={30} />
		</Link>
	);
};
