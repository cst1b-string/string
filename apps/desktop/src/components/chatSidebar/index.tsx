import { ChatButton } from '@/components/chatButton';

function getChatNames(): string[] {
	return ['David', 'Group Chat!'];
}

export default function ChatSidebar() {
	const chatNames = getChatNames();
	return (
		<div>
			<ChatButton chatName='New Chat +' />
			{chatNames.map((chatName, index) => (
				<ChatButton key={index} chatName={chatName} />
			))}
		</div>
	);
}
