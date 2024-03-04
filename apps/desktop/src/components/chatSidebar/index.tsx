"use client";

import { CircularChatButton } from "@/components/circularChatButton";
import { useRspc } from "@/integration";
import { Channel } from "@/integration/bindings";
import Link from "next/link";
import { useState } from "react";

export default function ChatSidebar({
	selectedChannel,
	setSelectedChannel,
}: {
	selectedChannel: number;
	setSelectedChannel: (channel: number) => void;
}) {
	const rspc = useRspc();
	// const { data } = rspc.useQuery(["channel.list"]);
	const channel1: Channel = {
		id: 1,
		title: "New Channel",
		networkId: 1,
	};
	const channel2: Channel = {
		id: 2,
		title: "Another Channel",
		networkId: 2,
	};
	const data: Channel[] = [channel1, channel2];

	// You can then use the newChannel object in your code

	if (data && selectedChannel < 0) {
		setSelectedChannel(data[0]!.id as number);
	}

	const handleChannelClick = (channel: Channel) => {
		console.log("Channel clicked: " + channel.id);
		setSelectedChannel(channel.id as number);
		console.log("Selected channel: " + selectedChannel);
	};

	const [filteredChannels, setFilteredChannels] = useState<Set<number>>(new Set<number>());

	const handleSearch = (event: React.ChangeEvent<HTMLInputElement>) => {
		var currFilteredChannels = new Set<number>();
		if (data) {
			const searchValue = event.target.value.toLowerCase();
			data.forEach((channel: Channel) => {
				if (!channel.title.toLowerCase().includes(searchValue)) {
					currFilteredChannels.add(channel.id);
				}
			});
		}
		setFilteredChannels(currFilteredChannels);
	};

	return (
		<div className="px-2 flex flex-col items-center no-scrollbar overflow-auto space-y-4 bg-darkGrey h-full w-20 py-2">
			<div className="grid grid-cols-5 space-x-1">
				<input
					id="search"
					type="text"
					placeholder="Search"
					className="px-1 py-1 rounded bg-buttonBlue text-white col-span-4"
					onChange={handleSearch}
				/>
				<Link href="./newChat">
					<button className="pl-1 bg-transparent hover:bg-hoverBlue text-white w-full py-1 rounded col-span-1">
						+
					</button>
				</Link>
			</div>
			{data &&
				data.map((channel) => (
					<div
						key={channel.id}
						className={` text-white w-full py-3 rounded ${
							filteredChannels.has(channel.id) ? "hidden" : ""
						} ${
							selectedChannel == channel.id ? "bg-buttonBlue" : "bg-transparent hover:bg-hoverBlue"
						}`}
						onClick={() => handleChannelClick(channel)}
					>
						<CircularChatButton chatName={channel.title} />
					</div>
				))}
		</div>
	);
}
