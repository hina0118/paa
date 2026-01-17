import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Input } from './input'

describe('Input', () => {
  it('renders input', () => {
    render(<Input />)
    const input = screen.getByRole('textbox')
    expect(input).toBeInTheDocument()
  })

  it('accepts text input', async () => {
    const user = userEvent.setup()
    render(<Input />)
    const input = screen.getByRole('textbox')

    await user.type(input, 'Hello World')
    expect(input).toHaveValue('Hello World')
  })

  it('handles onChange event', async () => {
    const handleChange = vi.fn()
    const user = userEvent.setup()

    render(<Input onChange={handleChange} />)
    const input = screen.getByRole('textbox')

    await user.type(input, 'a')
    expect(handleChange).toHaveBeenCalled()
  })

  it('renders with placeholder', () => {
    render(<Input placeholder="Enter text..." />)
    const input = screen.getByPlaceholderText('Enter text...')
    expect(input).toBeInTheDocument()
  })

  it('can be disabled', () => {
    render(<Input disabled />)
    const input = screen.getByRole('textbox')
    expect(input).toBeDisabled()
  })

  it('does not accept input when disabled', async () => {
    const user = userEvent.setup()
    render(<Input disabled />)
    const input = screen.getByRole('textbox')

    await user.type(input, 'test')
    expect(input).toHaveValue('')
  })

  it('supports different input types', () => {
    const { rerender } = render(<Input type="text" />)
    let input = screen.getByRole('textbox')
    expect(input).toHaveAttribute('type', 'text')

    rerender(<Input type="email" />)
    input = screen.getByRole('textbox')
    expect(input).toHaveAttribute('type', 'email')

    rerender(<Input type="password" />)
    // パスワードフィールドはtextboxロールを持たない
    const passwordInput = document.querySelector('input[type="password"]')
    expect(passwordInput).toBeInTheDocument()
  })

  it('applies default styling', () => {
    render(<Input />)
    const input = screen.getByRole('textbox')
    expect(input).toHaveClass('flex')
    expect(input).toHaveClass('h-9')
    expect(input).toHaveClass('w-full')
    expect(input).toHaveClass('rounded-md')
    expect(input).toHaveClass('border')
  })

  it('accepts custom className', () => {
    render(<Input className="custom-class" />)
    const input = screen.getByRole('textbox')
    expect(input).toHaveClass('custom-class')
    expect(input).toHaveClass('flex') // デフォルトクラスも保持
  })

  it('forwards ref', () => {
    const ref = { current: null }
    render(<Input ref={ref as any} />)
    expect(ref.current).not.toBeNull()
  })

  it('accepts initial value', () => {
    render(<Input value="Initial" onChange={() => {}} />)
    const input = screen.getByRole('textbox')
    expect(input).toHaveValue('Initial')
  })

  it('can be cleared', async () => {
    const user = userEvent.setup()
    render(<Input defaultValue="text" />)
    const input = screen.getByRole('textbox') as HTMLInputElement

    await user.clear(input)
    expect(input).toHaveValue('')
  })

  it('supports maxLength attribute', async () => {
    const user = userEvent.setup()
    render(<Input maxLength={5} />)
    const input = screen.getByRole('textbox')

    await user.type(input, 'abcdefgh')
    expect(input).toHaveValue('abcde')
  })

  it('supports required attribute', () => {
    render(<Input required />)
    const input = screen.getByRole('textbox')
    expect(input).toBeRequired()
  })
})
