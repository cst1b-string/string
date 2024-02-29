// import { ChatButton } from "@/components/chatButton";
import { CircularChatButton } from "@/components/circularChatButton";
import { faker } from "@faker-js/faker";

function getChatNames(): string[] {
	// return new Array(15).fill(true).map(() => faker.lorem.sentence({ min: 10, max: 30 }));
	return ["David", "Group Chat!", "random", "another server", "my server", "friends"];
}

export default function ChatSidebar() {
	const chatNames = getChatNames();
	return (
		// "no-scrollbar" defined in globals.css
		<div className="flex flex-col items-center no-scrollbar overflow-auto space-y-4 bg-darkGrey h-full w-20 pt-2 pb-2"> 
			{chatNames.map((chatName, index) => (
				<CircularChatButton key={index} chatName={chatName} />
			))}
			<CircularChatButton chatName = "+"/>
		</div>
	);
}
