import Link from "next/link";

export const NewChatButton = () => {
	return (
		<Link href="/chat">
			<button className="bg-[#3399FF] text-[white] w-full py-4">
				New Chat +
			</button>
		</Link>
	);
}
