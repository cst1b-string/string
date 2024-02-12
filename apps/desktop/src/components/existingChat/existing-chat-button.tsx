import Link from "next/link";

export const ExistingChatButton = () => {
	return (
		<Link href="/chat">
			<button className="bg-[#3363ff] text-[white] w-full py-4">
				Chat
			</button>
		</Link>
	);
}
