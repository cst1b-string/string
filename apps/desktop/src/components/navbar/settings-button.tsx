import Image from "next/image";
import Link from "next/link";

export const SettingsButton = () => (
	<Link href="/settings">
		<Image src="/settings-gear.png" alt="Settings" width={40} height={30} />
	</Link>
);
