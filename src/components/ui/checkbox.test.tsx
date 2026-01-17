import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Checkbox } from './checkbox'

describe('Checkbox', () => {
  it('renders checkbox', () => {
    render(<Checkbox />)
    const checkbox = screen.getByRole('checkbox')
    expect(checkbox).toBeInTheDocument()
  })

  it('handles checked state change', async () => {
    const handleChange = vi.fn()
    const user = userEvent.setup()

    render(<Checkbox onCheckedChange={handleChange} />)
    const checkbox = screen.getByRole('checkbox')

    await user.click(checkbox)
    expect(handleChange).toHaveBeenCalledWith(true)
  })

  it('can be initially checked', () => {
    render(<Checkbox checked={true} />)
    const checkbox = screen.getByRole('checkbox')
    expect(checkbox).toHaveAttribute('data-state', 'checked')
  })

  it('can be initially unchecked', () => {
    render(<Checkbox checked={false} />)
    const checkbox = screen.getByRole('checkbox')
    expect(checkbox).toHaveAttribute('data-state', 'unchecked')
  })

  it('applies default styling', () => {
    const { container } = render(<Checkbox />)
    const checkbox = container.querySelector('button[role="checkbox"]')
    expect(checkbox).toHaveClass('h-4')
    expect(checkbox).toHaveClass('w-4')
    expect(checkbox).toHaveClass('rounded-sm')
    expect(checkbox).toHaveClass('border')
  })

  it('accepts custom className', () => {
    const { container } = render(<Checkbox className="custom-class" />)
    const checkbox = container.querySelector('button[role="checkbox"]')
    expect(checkbox).toHaveClass('custom-class')
    expect(checkbox).toHaveClass('h-4') // デフォルトクラスも保持
  })

  it('can be disabled', () => {
    render(<Checkbox disabled />)
    const checkbox = screen.getByRole('checkbox')
    expect(checkbox).toBeDisabled()
  })

  it('does not trigger onChange when disabled', async () => {
    const handleChange = vi.fn()
    const user = userEvent.setup()

    render(<Checkbox disabled onCheckedChange={handleChange} />)
    const checkbox = screen.getByRole('checkbox')

    await user.click(checkbox)
    expect(handleChange).not.toHaveBeenCalled()
  })

  it('forwards ref', () => {
    const ref = { current: null }
    render(<Checkbox ref={ref as any} />)
    expect(ref.current).not.toBeNull()
  })

  it('toggles state on multiple clicks', async () => {
    const handleChange = vi.fn()
    const user = userEvent.setup()

    render(<Checkbox onCheckedChange={handleChange} />)
    const checkbox = screen.getByRole('checkbox')

    // 最初のクリック: チェック
    await user.click(checkbox)
    expect(handleChange).toHaveBeenNthCalledWith(1, true)

    // 2回目のクリック: チェック解除
    await user.click(checkbox)
    expect(handleChange).toHaveBeenNthCalledWith(2, false)
  })
})
