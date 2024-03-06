"use client";

import { CircularChatButton } from "@/components/circularChatButton";
import { useRspc } from "@/integration";
import { Channel } from "@/integration/bindings";
import { useEffect, useState } from "react";

export default function ChatSidebar({
	selectedChannel,
	setSelectedChannel,
}: {
	selectedChannel: number;
	setSelectedChannel: (channel: number) => void;
}) {
	const rspc = useRspc();
	const { data } = rspc.useQuery(["channel.list"]);

	const [isExpanded, setIsExpanded] = useState(false);
	const toggleExpansion = () => {
		setIsExpanded(!isExpanded);
	};
	const align = isExpanded ? "items-start" : "items-center";

	// const channel1: Channel = {
	// 	id: 1,
	// 	title: "New Channel",
	// 	networkId: 1,
	// };
	// const channel2: Channel = {
	// 	id: 2,
	// 	title: "Another Channel",
	// 	networkId: 2,
	// };
	// const data: Channel[] = [channel1, channel2];

	// You can then use the newChannel object in your code

	// runs only after the first render to set initial selected channel
	useEffect(() => {
		console.log("data: ", data);
		if (data && data.length > 0 && selectedChannel < 0) {
			console.log(data);
			setSelectedChannel(data[0]!.id);
		}
	}, []);

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
		<div
			className={`bg-darkSidebar h-full ${isExpanded ? "min-w-70" : "min-w-20"} grid grid-rows-[auto,1fr,auto]`}
		>
			<div
				className={`${isExpanded ? "grid grid-cols-5" : "flex justify-center"} space-x-1 h-[60px] py-2 px-2`}
			>
				<input
					id="search"
					type="text"
					placeholder="Search"
					className={`px-1 py-1 rounded ${
						isExpanded ? "" : "hidden"
					} bg-darkInput text-darkText col-span-4`}
					onChange={handleSearch}
				/>
				<button onClick={toggleExpansion} className="mt-auto mb-2 text-darkText font-bold text-2xl">
					{isExpanded ? "-" : "+"}
				</button>
			</div>
			<div className="px-2 py-2 flex flex-col items-center no-scrollbar overflow-auto space-y-1 bg-transparent">
				{data &&
					data.length > 0 &&
					data.map((channel) => (
						<div
							key={channel.id}
							className={` text-white w-full py-1 px-1 rounded ${
								filteredChannels.has(channel.id) ? "hidden" : ""
							} ${selectedChannel == channel.id ? "bg-darkSelected" : " hover:bg-darkHover"}`}
							onClick={() => handleChannelClick(channel)}
						>
							<CircularChatButton chatName={channel.title} isExpanded={isExpanded} />
						</div>
					))}
			</div>
			<div className={`bg-darkNewChat flex justify-center px-2 py-2 ${align}`}>
				<div className="w-full py-1 px-1 rounded hover:bg-darkHover">
					<CircularChatButton chatName="+" isExpanded={isExpanded} />
				</div>
			</div>
		</div>
	);
}
