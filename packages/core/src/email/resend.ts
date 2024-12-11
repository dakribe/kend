import { Resend } from "resend";
import { Schema } from "effect";

const resend = new Resend(process.env.RESEND_API_KEY!);

const SendEmailSchema = Schema.Struct({
	from: Schema.String,
	to: Schema.String,
	subject: Schema.String,
	html: Schema.String,
});

type SendEmail = Schema.Schema.Type<typeof SendEmailSchema>;

export async function sendEmail(params: SendEmail) {
	const { error } = await resend.emails.send({
		from: params.from,
		to: params.to,
		subject: params.subject,
		html: params.html,
	});

	// TODO: handle error with effect
	if (error) {
		return console.log(error);
	}
}
