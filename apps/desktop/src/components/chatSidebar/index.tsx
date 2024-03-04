import { CircularChatButton } from "@/components/circularChatButton";
import React, { useState } from "react";

function getChatNames(): string[] {
	// return new Array(15).fill(true).map(() => faker.lorem.sentence({ min: 10, max: 30 }));
	// return ["David", "Group Chat!", "random", "another server", "my server", "friends"];
	return [
		"David",
		"Group Chat!",
		"random",
		"another server",
		"my server",
		"friends",
		"Slack",
		"Discord",
		"College",
		"McDonalds",
		"RUSTT",
	];
}

export default function ChatSidebar() {
	const [isExpanded, setIsExpanded] = useState(false);

	const toggleExpansion = () => {
		setIsExpanded(!isExpanded);
	};

	const chatNames = getChatNames();

	const align = isExpanded ? "items-start ml-3" : "items-center";
	
	return (
		// "no-scrollbar" defined in globals.css
		<div className={`h-full ${isExpanded ? "min-w-70" : "min-w-20"}`}>
			<div className="flex flex-col justify-between bg-darkGrey pt-2 pb-2 h-full">
				<div className="flex flex-col items-center pt-2">
					<button onClick={toggleExpansion} className="mt-auto mb-2 text-white font-bold text-2xl">
						{isExpanded ? "-" : "+"}
					</button>
				</div>
				<div className={`flex flex-col ${align} no-scrollbar overflow-auto space-y-4`}>
					{chatNames.map((chatName, index) => (
						<CircularChatButton key={index} chatName={chatName} isExpanded={isExpanded} />
					))}
				</div>
				<div className={`flex flex-col items-start pt-2 ${align}`}>
					<CircularChatButton chatName="+" isExpanded={isExpanded} />
				</div>
			</div>
		</div>
	);
}
