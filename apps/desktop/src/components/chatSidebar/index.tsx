"use client";

import { useRspc } from "@/integration";
import { Channel } from "@/integration/bindings";
import Link from "next/link";

export default function ChatSidebar({
	selectedChannel,
	setSelectedChannel,
}: {
	selectedChannel: number;
	setSelectedChannel: (channel: number) => void;
}) {
	const rspc = useRspc();
	const { data } = rspc.useQuery(["channel.list"]);

	if (data) {
		setSelectedChannel(data[0]!.id as number);
	}

	const handleChannelClick = (channel: Channel) => {
		setSelectedChannel(channel.id as number);
	};

	return (
		<div>
			<Link href="./newChat">
				<button className="bg-transparent hover:bg-[#4e627a] text-white w-full py-5 rounded">
					New Chat
				</button>
			</Link>
			{data &&
				data.map((channel) => (
					<button
						key={channel.id}
						className={`bg-transparent hover:bg-[#4e627a] text-white w-full py-5 rounded ${
							selectedChannel === channel.id ? "bg-[#4e627a]" : ""
						}`}
						onClick={() => handleChannelClick(channel)}
					>
						{channel.title}
					</button>
				))}
		</div>
	);
}
