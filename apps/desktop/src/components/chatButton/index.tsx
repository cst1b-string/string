import { Link } from "react-router-dom";

export const ChatButton = ({ chatName }: { chatName: string }) => (
	<Link to="./newChat">
		<button className="bg-transparent hover:bg-[#4e627a] text-white w-full py-5 rounded">
			{chatName}
		</button>
	</Link>
);
