import { useRspc } from "@/integration";
import Image from "next/image";
import Link from "next/link";

export const SettingsButton = () => {
	const rspc = useRspc();
	// const hasAccount = rspc.useQuery(["account.login", null]).data;

	// if (!hasAccount) {
	// 	return null;
	// }
	return (
		<Link href="/settings">
			<Image src="/settings-gear.png" alt="Settings" width={40} height={30} />
		</Link>
	);
};
