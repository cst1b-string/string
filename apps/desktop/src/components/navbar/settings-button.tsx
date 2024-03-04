import Image from "next/image";
import Link from "next/link";

var hasAccount = true;

export const SettingsButton = () => {
	if (!hasAccount) {
		return null;
	}
	return (
		<Link href="/settings">
			<Image src="/settings-gear.png" alt="Settings" width={40} height={30} />
		<Link href="/settings">
			<Image src="/settings-gear.png" alt="Settings" width={40} height={30} />
		</Link>
	);
};
