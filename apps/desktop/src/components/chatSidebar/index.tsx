// import { ChatButton } from "@/components/chatButton";
import { CircularChatButton } from "@/components/circularButton";
import { faker } from "@faker-js/faker";

function getChatNames(): string[] {
	return new Array(10).fill(true).map(() => faker.lorem.sentence({ min: 10, max: 30 }));
	// return ["David", "Group Chat!"];
}

export default function ChatSidebar() {
	const chatNames = getChatNames();
	return (
		// "no-scrollbar" defined in globals.css
		<div className="flex flex-col items-center bg-darkGrey h-full overflow-auto no-scrollbar w-20 space-y-4 pt-2"> 
			{chatNames.map((chatName, index) => (
				<CircularChatButton key={index} chatName={chatName} />
			))}
			<CircularChatButton chatName = "+"/>
		</div>
	);	
}
