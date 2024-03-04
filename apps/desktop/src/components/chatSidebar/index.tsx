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

	const [isExpanded, setIsExpanded] = useState(false);
	const toggleExpansion = () => {
		setIsExpanded(!isExpanded);
	};
	const align = isExpanded ? "items-start" : "items-center";

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
		<div className={`h-full ${isExpanded ? "min-w-70" : "min-w-20"}`}>
			<div className="px-2 flex flex-col items-center no-scrollbar overflow-auto space-y-1 bg-transparent h-full py-2">
				<div className={`${isExpanded ? "grid grid-cols-5" : "flex justify-center"} space-x-1`}>
					<input
						id="search"
						type="text"
						placeholder="Search"
						className={`px-1 py-1 rounded ${
							isExpanded ? "" : "hidden"
						} bg-buttonBlue text-white col-span-4`}
						onChange={handleSearch}
					/>
					<button onClick={toggleExpansion} className="mt-auto mb-2 text-white font-bold text-2xl">
						{isExpanded ? "-" : "+"}
					</button>
				</div>
				{data &&
					data.map((channel) => (
						<div
							key={channel.id}
							className={` text-white w-full py-1 px-1 rounded ${
								filteredChannels.has(channel.id) ? "hidden" : ""
							} ${
								selectedChannel == channel.id
									? "bg-buttonBlue"
									: "bg-transparent hover:bg-hoverBlue"
							}`}
							onClick={() => handleChannelClick(channel)}
						>
							<CircularChatButton chatName={channel.title} isExpanded={isExpanded} />
						</div>
					))}
				<div className={`flex flex-col pt-2 ${align}`}>
					<CircularChatButton chatName="+" isExpanded={isExpanded} />
				</div>
			</div>
		</div>
	);
}
