import Link from "next/link";

export const ChatButton = () => {
	return (
		<Link href="/chat">
			<button className="bg-[#3399FF] text-[white] w-full py-4">
				New Chat +
			</button>
		</Link>
	);
}
